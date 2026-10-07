use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError, Weak};

use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::Difficulty;

use crate::entity::ai::control::drowned_move_control::DrownedMoveControl;
use crate::entity::ai::goal::active_target::ActiveTargetGoal;
use crate::entity::ai::goal::drowned_go_to_beach::DrownedGoToBeachGoal;
use crate::entity::ai::goal::drowned_go_to_water::DrownedGoToWaterGoal;
use crate::entity::ai::goal::drowned_swim_up::DrownedSwimUpGoal;
use crate::entity::ai::goal::drowned_trident_attack::DrownedTridentAttackGoal;
use crate::entity::ai::goal::swim::SwimGoal;
use crate::entity::ai::goal::zombie_attack::ZombieAttackGoal;
use crate::entity::ai::goal::Goal;
use crate::entity::ai::pathfinder::Navigator;
use crate::entity::ai::pathfinder::node::PathType;
use crate::entity::living::LivingEntity;
use crate::entity::mob::equipment::RegionalDifficulty;
use crate::entity::mob::spawn::SpawnGroupData;
use crate::entity::mob::zombie::ZombieEntityBase;
use crate::entity::mob::{Mob, MobEntity, RangedAttackMob};
use crate::entity::projectile::arrow::ArrowPickup;
use crate::entity::projectile::trident::TridentEntity;
use crate::entity::{Entity, EntityBase};
use crate::world::World;

/// Vanilla `Drowned.travelInWater`.
///
/// Under the surface with a reason to swim, a drowned moves itself at a fiftieth of
/// the walking pace, carries its momentum and takes on no gravity at all, so it holds
/// whatever depth it chose. Anything else — in water with no reason to swim, or out of
/// it — declines and the ordinary physics run, which is what settles an idle drowned
/// on the bottom.
fn travel(mob: &dyn Mob, caller: &dyn EntityBase) -> bool {
    let mob_entity = mob.get_mob_entity();
    let living = &mob_entity.living_entity;
    let entity = &living.entity;

    if !(entity.is_submerged_in_water() && mob.wants_to_swim()) {
        return false;
    }

    entity.update_velocity_from_input(living.movement_input.load(), 0.01);
    living.make_move(caller);
    entity.velocity.store(entity.velocity.load() * 0.9);

    true
}

/// Vanilla `Drowned.okTarget`: in daylight a drowned only cares about what is in the
/// water with it.
fn ok_target(mob: &dyn Mob) -> bool {
    let target = mob
        .get_mob_entity()
        .target
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();

    let Some(target) = target else {
        return false;
    };

    !mob.get_entity().world.load().is_day()
        || target.get_entity().touching_water.load(Ordering::Relaxed)
}

/// The same rule as [`ok_target`] for the target goals, which are handed the candidate
/// rather than the mob.
fn gated_to_water_or_night(target: &LivingEntity, world: &World) -> bool {
    target.entity.touching_water.load(Ordering::Relaxed) || !world.is_day()
}

/// Vanilla `Turtle.BABY_ON_LAND_SELECTOR`: a drowned only hunts turtle hatchlings that
/// are out of the water.
fn baby_turtle_on_land(target: &LivingEntity, _world: &World) -> bool {
    target.entity.age.load(Ordering::Relaxed) < 0
        && !target.entity.touching_water.load(Ordering::Relaxed)
}

pub struct DrownedEntity {
    entity: Arc<ZombieEntityBase>,
    /// Vanilla `Drowned.searchingForLand`, set while the swim-up and beach goals want
    /// it rising towards a shore. It feeds `wantsToSwim`, so it lives on the entity
    /// rather than inside either goal.
    searching_for_land: AtomicBool,
}

impl DrownedEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        Self::create(entity, false)
    }

    #[must_use]
    pub fn with_can_break_doors(entity: Entity, can_break_doors: bool) -> Arc<Self> {
        Self::create(entity, can_break_doors)
    }

    fn create(entity: Entity, can_break_doors: bool) -> Arc<Self> {
        let drowned = Arc::new(Self {
            entity: ZombieEntityBase::with_can_break_doors(entity, can_break_doors),
            searching_for_land: AtomicBool::new(false),
        });

        // Vanilla `DrownedEntity` constructor: amphibious navigation, water as cheap to
        // path through as land.
        {
            let mut navigator = Navigator::amphibious(false);
            navigator.set_pathfinding_malus(PathType::Water, 0.0);
            // A drowned swims, so the navigator treats water as its element even though
            // its float goal is gone.
            navigator.set_can_float(true);
            *drowned
                .entity
                .mob_entity
                .navigator
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = navigator;
        }

        // Vanilla `Drowned.DrownedMoveControl`: the drowned steers its own swimming
        // rather than borrowing the walking controller.
        *drowned
            .entity
            .mob_entity
            .move_control
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Box::new(DrownedMoveControl::default());

        let ranged: Weak<dyn RangedAttackMob> = {
            let ranged: Arc<dyn RangedAttackMob> = drowned.clone();
            Arc::downgrade(&ranged)
        };
        drowned.init_drowned_goals(ranged);

        drowned
    }

    /// Vanilla `DrownedEntity.addBehaviourGoals`, on top of the zombie set the base
    /// installed, plus the target list that replaces the zombie's.
    fn init_drowned_goals(&self, ranged: Weak<dyn RangedAttackMob>) {
        let mob: &dyn Mob = self;
        let mob_entity = &self.entity.mob_entity;

        let mut stopped_goals = {
            let mut goal_selector = mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(PoisonError::into_inner);

            // Vanilla's drowned does not float: it sinks unless it has a reason to
            // swim, so the zombie's float goal comes off and the move control plus the
            // water travel do the work instead.
            let mut stopped = goal_selector.remove_goals::<SwimGoal>();

            // Vanilla adds the water goal first, then the trident throw, then the melee
            // at the same priority — ties go to whichever was added first, as in the
            // goal selector's iteration order.
            goal_selector.add_goal(1, DrownedGoToWaterGoal::new(1.0));
            goal_selector.add_goal(
                2,
                Box::new(DrownedTridentAttackGoal::new(ranged, 1.0, 40, 10.0)),
            );
            stopped.extend(goal_selector.remove_goals::<ZombieAttackGoal>());
            goal_selector.add_goal(2, ZombieAttackGoal::new(1.0, false).gated_by(ok_target));
            goal_selector.add_goal(5, DrownedGoToBeachGoal::new(1.0));
            goal_selector.add_goal(6, DrownedSwimUpGoal::new(1.0));

            // The plain stroll at 7 stays as the zombie installed it: vanilla's
            // day/night instinct lives in the goals above, not in where it wanders.
            stopped
        };

        let mut stopped_targets = {
            let mut target_selector = mob_entity
                .target_selector
                .lock()
                .unwrap_or_else(PoisonError::into_inner);

            let stopped = target_selector.remove_goals::<ActiveTargetGoal>();

            // Vanilla `DrownedEntity` target list: players only while it is dark or
            // they are wet, merchants and golems and axolotls unconditionally, and
            // turtle hatchlings last.
            target_selector.add_goal(
                2,
                Box::new(ActiveTargetGoal::new(
                    mob_entity,
                    &EntityType::PLAYER,
                    10,
                    true,
                    false,
                    Some(gated_to_water_or_night),
                )),
            );
            target_selector.add_goal(
                3,
                Box::new(ActiveTargetGoal::new(
                    mob_entity,
                    &EntityType::VILLAGER,
                    10,
                    false,
                    false,
                    None::<fn(&LivingEntity, &World) -> bool>,
                )),
            );
            target_selector.add_goal(
                3,
                Box::new(ActiveTargetGoal::new(
                    mob_entity,
                    &EntityType::WANDERING_TRADER,
                    10,
                    false,
                    false,
                    None::<fn(&LivingEntity, &World) -> bool>,
                )),
            );
            target_selector.add_goal(
                3,
                Box::new(ActiveTargetGoal::new(
                    mob_entity,
                    &EntityType::IRON_GOLEM,
                    10,
                    true,
                    false,
                    None::<fn(&LivingEntity, &World) -> bool>,
                )),
            );
            target_selector.add_goal(
                3,
                Box::new(ActiveTargetGoal::new(
                    mob_entity,
                    &EntityType::AXOLOTL,
                    10,
                    true,
                    false,
                    None::<fn(&LivingEntity, &World) -> bool>,
                )),
            );
            target_selector.add_goal(
                5,
                Box::new(ActiveTargetGoal::new(
                    mob_entity,
                    &EntityType::TURTLE,
                    10,
                    true,
                    false,
                    Some(baby_turtle_on_land),
                )),
            );

            stopped
        };

        for goal in &mut stopped_goals {
            goal.stop(mob);
        }
        for goal in &mut stopped_targets {
            goal.stop(mob);
        }
    }

    /// Vanilla `DrownedEntity.performRangedAttack`: throw a copy of the held trident,
    /// aimed a little above the target so the arc drops onto it. Vanilla keeps the
    /// speed fixed and lets the difficulty decide how wide the shots scatter.
    pub fn throw_trident(&self, target: &Arc<dyn EntityBase>, _power: f32) {
        let entity = self.get_entity();
        let world = entity.world.load_full();

        let trident_entity = Entity::new(world.clone(), entity.pos.load(), &EntityType::TRIDENT);
        let stack = ItemStack::new(1, &Item::TRIDENT);
        let trident =
            TridentEntity::new_shot(trident_entity, entity, stack.clone(), ArrowPickup::Allowed);

        let source = trident.get_entity().pos.load();
        let target_entity = target.get_entity();
        let target_pos = target_entity.pos.load();
        let dx = target_pos.x - source.x;
        let dy = (target_pos.y + f64::from(target_entity.entity_dimension.load().height) / 3.0)
            - source.y;
        let dz = target_pos.z - source.z;
        // Vanilla `shoot(dx, dy + horizontal * 0.2, dz, 1.6, 14 - difficulty * 4)`.
        let divergence = f64::from(14 - (world.level_info.load().difficulty as i32) * 4);

        trident.set_velocity(dx, dy + dx.hypot(dz) * 0.2, dz, 1.6, divergence);
        trident.apply_on_projectile_spawned(&stack);
        world.spawn_entity(Arc::new(trident));
    }
}

impl Mob for DrownedEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.entity.mob_entity
    }

    fn spawn_as_baby(&self) -> bool {
        self.entity.spawn_as_baby()
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        self.entity.mob_write_nbt(nbt);
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        self.entity.mob_read_nbt(nbt);
    }

    /// Vanilla `Drowned.wantsToSwim`: it swims at whatever it is chasing that is in the
    /// water, and while it is looking for a shore. Otherwise it stops fighting the
    /// water and settles.
    fn wants_to_swim(&self) -> bool {
        if self.searching_for_land.load(Ordering::Relaxed) {
            return true;
        }

        self.get_mob_entity()
            .target
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .is_some_and(|target| target.get_entity().touching_water.load(Ordering::Relaxed))
    }

    fn is_searching_for_land(&self) -> bool {
        self.searching_for_land.load(Ordering::Relaxed)
    }

    fn set_searching_for_land(&self, searching: bool) {
        self.searching_for_land.store(searching, Ordering::Relaxed);
    }

    /// Vanilla `Drowned.travelInWater`.
    fn custom_travel(&self, caller: &dyn EntityBase) -> bool {
        travel(self, caller)
    }

    /// Vanilla `Drowned.isPushedByFluid` is `!isSwimming`; this port only tracks the
    /// swimming flag for players, so a drowned simply is never pushed.
    fn mob_is_pushed_by_fluids(&self) -> bool {
        false
    }

    fn finalize_spawn(
        &self,
        world: &Arc<World>,
        group_data: Option<SpawnGroupData>,
    ) -> Option<SpawnGroupData> {
        self.get_mob_entity().finalize_spawn_base();

        // Vanilla `Zombie` decides door breaking when the mob spawns, by rolling the
        // clamped local difficulty; `BreakDoorGoal` itself refuses anything below Hard.
        let difficulty = RegionalDifficulty::at(world, self.get_entity().pos.load());
        if difficulty.base_difficulty == Difficulty::Hard
            && rand::random::<f32>() < difficulty.special_multiplier * 0.1
        {
            self.entity.set_can_break_doors(true, self);
        }

        group_data
    }
}

impl RangedAttackMob for DrownedEntity {
    fn perform_ranged_attack(&self, target: &Arc<dyn EntityBase>, power: f32) {
        self.throw_trident(target, power);
    }
}
