use super::{Mob, MobEntity};
use crate::entity::ai::goal::break_door::BreakDoorGoal;
use crate::entity::ai::goal::destroy_egg::DestroyEggGoal;
use crate::entity::ai::goal::look_around::RandomLookAroundGoal;
use crate::entity::ai::goal::revenge::RevengeGoal;
use crate::entity::ai::goal::swim::SwimGoal;
use crate::entity::ai::goal::wander_around::WanderAroundGoal;
use crate::entity::ai::goal::zombie_attack::ZombieAttackGoal;
use crate::entity::mob::equipment::RegionalDifficulty;
use crate::entity::{
    Entity, EntityBase,
    ai::goal::{Goal, active_target::ActiveTargetGoal, look_at_entity::LookAtEntityGoal},
};
use crate::world::World;
use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::Difficulty;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Weak};

pub mod drowned;
pub mod husk;
#[allow(clippy::module_inception)]
pub mod zombie;
pub mod zombie_villager;

pub struct ZombieEntityBase {
    pub mob_entity: MobEntity,
    pub can_break_doors: AtomicBool,
    pub is_baby: AtomicBool,
    /// Vanilla `Zombie.inWaterTime`: ticks spent with the eyes underwater; a
    /// dry tick resets it to `-1`.
    in_water_time: AtomicI32,
    /// Vanilla `Zombie.conversionTime`: ticks left until the water
    /// conversion finishes, `-1` when not converting.
    conversion_time: AtomicI32,
}

impl ZombieEntityBase {
    pub fn new(entity: Entity) -> Arc<Self> {
        Self::with_can_break_doors(entity, false)
    }

    pub fn with_can_break_doors(entity: Entity, can_break_doors: bool) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let zombie = Self {
            mob_entity,
            can_break_doors: AtomicBool::new(can_break_doors),
            is_baby: AtomicBool::new(false),
            in_water_time: AtomicI32::new(-1),
            conversion_time: AtomicI32::new(-1),
        };
        let mob_arc = Arc::new(zombie);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut target_selector = mob_arc
                .mob_entity
                .target_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
            if can_break_doors {
                goal_selector.add_goal(1, Box::new(BreakDoorGoal::default()));
            }
            goal_selector.add_goal(2, ZombieAttackGoal::new(1.0, false));
            goal_selector.add_goal(4, DestroyEggGoal::new(1.0, 3));
            goal_selector.add_goal(7, Box::new(WanderAroundGoal::new(1.0)));
            goal_selector.add_goal(
                8,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 8.0),
            );
            goal_selector.add_goal(8, Box::new(RandomLookAroundGoal::default()));

            target_selector.add_goal(
                1,
                Box::new(
                    RevengeGoal::new(true).alerting_others_except(|entity_type| {
                        entity_type == &EntityType::ZOMBIFIED_PIGLIN
                    }),
                ),
            );
            target_selector.add_goal(
                2,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::PLAYER, true),
            );
            target_selector.add_goal(
                3,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::VILLAGER, true),
            );
            // Vanilla targets `AbstractVillager`, which includes traders.
            target_selector.add_goal(
                3,
                ActiveTargetGoal::with_default(
                    &mob_arc.mob_entity,
                    &EntityType::WANDERING_TRADER,
                    true,
                ),
            );
            target_selector.add_goal(
                3,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::IRON_GOLEM, true),
            );
            target_selector.add_goal(
                5,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::TURTLE, true),
            );
        };

        mob_arc
    }

    #[must_use]
    pub fn can_break_doors(&self) -> bool {
        self.can_break_doors.load(Ordering::Relaxed)
    }

    pub fn set_can_break_doors(&self, can_break_doors: bool, mob: &dyn Mob) {
        if self
            .can_break_doors
            .swap(can_break_doors, Ordering::Relaxed)
            != can_break_doors
        {
            let mut stopped = {
                let mut goal_selector = self
                    .mob_entity
                    .goals_selector
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if can_break_doors {
                    goal_selector.add_goal(1, Box::new(BreakDoorGoal::default()));
                    Vec::new()
                } else {
                    goal_selector.remove_goals::<BreakDoorGoal>()
                }
            };
            for goal in &mut stopped {
                goal.stop(mob);
            }
        }
    }
}

impl ZombieEntityBase {
    /// Vanilla `Zombie.setBaby`: the baby state is only the synced flag, it never ages up.
    // TODO: vanilla baby zombies get the +50% SPEED_MODIFIER_BABY and 2.5x XP
    // (`Zombie.getBaseExperienceReward`), unlike passive babies which drop none.
    pub fn set_baby(&self, baby: bool) {
        self.mob_entity.set_baby_flag(
            &self.is_baby,
            pumpkin_data::tracked_data::zombie::BABY,
            baby,
        );
    }

    #[must_use]
    pub fn is_baby(&self) -> bool {
        self.is_baby.load(Ordering::Relaxed)
    }

    /// Vanilla `Zombie.tick` water half: after 600 ticks with the eyes
    /// underwater a 300 tick countdown to the conversion starts.
    pub fn tick_water_conversion(
        &self,
        caller: &dyn crate::entity::EntityBase,
        converts_in_water: bool,
    ) {
        let entity = &self.mob_entity.living_entity.entity;
        if !entity.is_alive() || self.mob_entity.is_no_ai() {
            return;
        }
        if self.conversion_time.load(Ordering::Relaxed) >= 0 {
            let remaining = self.conversion_time.fetch_sub(1, Ordering::Relaxed) - 1;
            if remaining < 0 {
                self.do_under_water_conversion(caller);
            }
        } else if converts_in_water && entity.is_submerged_in_water() {
            let time = self.in_water_time.fetch_add(1, Ordering::Relaxed) + 1;
            if time >= 600 {
                self.start_under_water_conversion(300);
            }
        } else {
            self.in_water_time.store(-1, Ordering::Relaxed);
        }
    }

    fn start_under_water_conversion(&self, ticks: i32) {
        self.conversion_time.store(ticks, Ordering::Relaxed);
        self.mob_entity.living_entity.entity.set_synced_data(
            pumpkin_data::tracked_data::zombie::CONVERTING_IN_WATER,
            true,
        );
    }

    /// Vanilla `Zombie.doUnderWaterConversion`; `target` is what this zombie
    /// family member becomes (husks turn into plain zombies).
    fn do_under_water_conversion(&self, caller: &dyn crate::entity::EntityBase) {
        let target = if caller.get_entity().entity_type == &EntityType::HUSK {
            &EntityType::ZOMBIE
        } else {
            &EntityType::DROWNED
        };
        self.convert_to_zombie_type(caller, target);
        let entity = &self.mob_entity.living_entity.entity;
        let world = entity.world.load();
        if !entity.is_silent() {
            let event = if target == &EntityType::ZOMBIE {
                pumpkin_data::world::WorldEvent::SoundHuskToZombie
            } else {
                pumpkin_data::world::WorldEvent::SoundZombieToDrowned
            };
            world.sync_world_event(event, entity.block_pos.load(), 0);
        }
    }

    /// Vanilla `Zombie.convertToZombieType`: replace this mob with another
    /// zombie-family type, carrying position, health and equipment over.
    fn convert_to_zombie_type(
        &self,
        caller: &dyn crate::entity::EntityBase,
        target: &'static EntityType,
    ) {
        let entity = &self.mob_entity.living_entity.entity;
        let world = entity.world.load();

        let replacement = crate::entity::r#type::from_type(
            target,
            entity.pos.load(),
            &world,
            uuid::Uuid::new_v4(),
        );
        let new_entity = replacement.get_entity();
        new_entity.set_rotation(entity.yaw.load(), entity.pitch.load());
        new_entity.head_yaw.store(entity.head_yaw.load());
        new_entity.velocity.store(entity.velocity.load());
        new_entity.invulnerable.store(
            entity.invulnerable.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        if let Some(custom_name) = &**entity.custom_name.load() {
            new_entity.set_custom_name(custom_name.clone());
            new_entity.set_custom_name_visible(entity.custom_name_visible.load(Ordering::Relaxed));
        }

        if let Some(mob) = replacement.get_mob() {
            mob.get_mob_entity().set_no_ai(self.mob_entity.is_no_ai());
            mob.get_mob_entity().persistence_required.store(
                self.mob_entity.persistence_required.load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
            // Vanilla `convertTo(…, true)` hands the equipment over.
            for slot in [
                EquipmentSlot::MAIN_HAND,
                EquipmentSlot::OFF_HAND,
                EquipmentSlot::HEAD,
                EquipmentSlot::CHEST,
                EquipmentSlot::LEGS,
                EquipmentSlot::FEET,
            ] {
                let stack = self
                    .mob_entity
                    .living_entity
                    .entity_equipment
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(&slot);
                if !stack.is_empty() {
                    mob.get_mob_entity()
                        .living_entity
                        .entity_equipment
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .put(&slot, stack);
                }
            }
            mob.get_mob_entity()
                .living_entity
                .health
                .store(self.mob_entity.living_entity.health.load());
            if let Some(zombie) = mob.as_zombie() {
                zombie.set_baby(self.is_baby());
                zombie.set_can_break_doors(self.can_break_doors(), mob);
            }
            mob.finalize_spawn(&world, None);
        }

        world.spawn_entity(replacement.clone());
        entity.remove();
    }

    /// Vanilla `Zombie.finalizeSpawn`: pickup-loot, baby and door-breaking
    /// rolls, equipment and the attribute randomisation.
    pub fn finalize_zombie_spawn(
        &self,
        world: &Arc<World>,
        supports_break_door_goal: bool,
    ) -> Option<crate::entity::mob::spawn::SpawnGroupData> {
        self.mob_entity.finalize_spawn_base();

        let difficulty =
            RegionalDifficulty::at(world, self.mob_entity.living_entity.entity.pos.load());
        let special = difficulty.special_multiplier;

        // Vanilla rolls `setCanPickUpLoot(random.nextFloat() < 0.55F * special)`.
        self.mob_entity
            .set_can_pick_up_loot(rand::random::<f32>() < 0.55 * special);
        if rand::random::<f32>() < 0.05 {
            self.set_baby(true);
            // TODO: vanilla baby zombies ride chickens (`ChickenJockey`).
        }
        self.set_can_break_doors(
            supports_break_door_goal && rand::random::<f32>() < special * 0.1,
            self,
        );
        self.populate_default_equipment_slots(world, &difficulty);
        self.populate_default_equipment_enchantments(&difficulty);
        // Vanilla's October 31st pumpkin head roll needs a wall-clock date,
        // which the server does not track.
        self.handle_attributes(special);
        None
    }

    /// Vanilla `Zombie.handleAttributes`: reinforcement chance base, knockback
    /// and follow-range bonuses, and the rare door-breaking leader zombie.
    fn handle_attributes(&self, special: f32) {
        let mut attributes = self
            .mob_entity
            .living_entity
            .attributes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if let Some(reinforcements) = attributes
            .get_mut(&pumpkin_data::attributes::Attributes::ZOMBIE_SPAWN_REINFORCEMENTS.id)
        {
            reinforcements.base_value = rand::random::<f64>() * 0.1;
            reinforcements.modifiers.clear();
        }
        if let Some(knockback) = attributes
            .get_mut(&pumpkin_data::attributes::Attributes::GENERIC_KNOCKBACK_RESISTANCE.id)
        {
            knockback.add_or_replace_modifier(crate::entity::attributes::Modifier {
                id: "minecraft:zombie_random_spawn_bonus".to_string(),
                amount: f64::from(rand::random::<f32>() * 0.05),
                operation: crate::entity::attributes::ModifierOperation::Add,
            });
        }
        let follow_bonus = f64::from(rand::random::<f32>() * 1.5 * special);
        if follow_bonus > 1.0
            && let Some(follow_range) =
                attributes.get_mut(&pumpkin_data::attributes::Attributes::GENERIC_FOLLOW_RANGE.id)
        {
            follow_range.add_or_replace_modifier(crate::entity::attributes::Modifier {
                id: "minecraft:zombie_random_spawn_bonus".to_string(),
                amount: follow_bonus,
                operation: crate::entity::attributes::ModifierOperation::MultiplyTotal,
            });
        }
        if rand::random::<f32>() < special * 0.05 {
            if let Some(reinforcements) = attributes
                .get_mut(&pumpkin_data::attributes::Attributes::ZOMBIE_SPAWN_REINFORCEMENTS.id)
            {
                reinforcements.add_or_replace_modifier(crate::entity::attributes::Modifier {
                    id: "minecraft:leader_zombie_bonus".to_string(),
                    amount: rand::random::<f64>() * 0.25 + 0.5,
                    operation: crate::entity::attributes::ModifierOperation::Add,
                });
            }
            if let Some(max_health) =
                attributes.get_mut(&pumpkin_data::attributes::Attributes::GENERIC_MAX_HEALTH.id)
            {
                max_health.add_or_replace_modifier(crate::entity::attributes::Modifier {
                    id: "minecraft:leader_zombie_bonus".to_string(),
                    amount: rand::random::<f64>() * 3.0 + 1.0,
                    operation: crate::entity::attributes::ModifierOperation::MultiplyTotal,
                });
            }
            // Leader zombies always break doors, whatever they rolled above.
            self.set_can_break_doors(true, self);
        }
    }

    /// Vanilla `Zombie.killedEntity`: on normal and hard, a fatal hit on a
    /// villager infects it instead of killing it.
    pub fn killed_entity_villager(
        &self,
        world: &Arc<World>,
        target: &Arc<dyn crate::entity::EntityBase>,
    ) -> bool {
        let difficulty = world.level_info.load().difficulty;
        if !matches!(difficulty, Difficulty::Normal | Difficulty::Hard)
            || target.get_entity().entity_type != &EntityType::VILLAGER
        {
            return false;
        }
        if difficulty != Difficulty::Hard && rand::random::<f32>() < 0.5 {
            return false;
        }

        let entity = &self.mob_entity.living_entity.entity;
        let zombie_villager =
            crate::entity::mob::zombie::zombie_villager::ZombieVillagerEntity::new(Entity::new(
                world.clone(),
                entity.pos.load(),
                &EntityType::ZOMBIE_VILLAGER,
            ));
        let new_entity = zombie_villager.get_entity();
        new_entity.set_rotation(entity.yaw.load(), entity.pitch.load());
        new_entity.head_yaw.store(entity.head_yaw.load());
        new_entity.velocity.store(entity.velocity.load());
        if self.is_baby() {
            zombie_villager.mob_entity.set_baby(true);
        }
        zombie_villager
            .get_mob_entity()
            .set_no_ai(self.mob_entity.is_no_ai());

        // The zombie villager keeps the villager's type, profession, level,
        // trades, gossips and experience for its cure.
        let mut villager_nbt = pumpkin_nbt::compound::NbtCompound::new();
        target.write_nbt(&mut villager_nbt);
        zombie_villager.mob_read_nbt(&villager_nbt);

        world.spawn_entity(zombie_villager.clone());
        target.get_entity().remove();
        if !entity.is_silent() {
            world.sync_world_event(
                pumpkin_data::world::WorldEvent::SoundZombieInfected,
                entity.block_pos.load(),
                0,
            );
        }
        true
    }

    /// Vanilla `Zombie.doHurtTarget`: a burning zombie with an empty hand can
    /// set its victim alight.
    pub fn after_melee_hit_burn(&self, target: &dyn crate::entity::EntityBase) {
        let entity = &self.mob_entity.living_entity.entity;
        let hand = self
            .mob_entity
            .living_entity
            .entity_equipment
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&pumpkin_data::data_component_impl::EquipmentSlot::MAIN_HAND);
        let burning = entity.fire_ticks.load(Ordering::Relaxed) > 0;
        if !hand.is_empty() || !burning {
            return;
        }
        let world = entity.world.load_full();
        let effective = RegionalDifficulty::at(&world, entity.pos.load()).effective_difficulty;
        if rand::random::<f32>() < effective * 0.3 {
            target.set_on_fire_for(2.0 * effective);
        }
    }

    /// Vanilla `Zombie.hurt` reinforcement half: on hard, a damaged zombie can
    /// call another one in. Only the zombie's current target is inherited;
    /// vanilla also falls back to the damager, which the revenge goal sets a
    /// tick later anyway.
    #[allow(clippy::too_many_lines)]
    pub fn on_damage_reinforcements(&self) {
        let entity = &self.mob_entity.living_entity.entity;
        let world = entity.world.load();
        // TODO: vanilla also honours the `doMobSpawning` game rule here.
        if world.level_info.load().difficulty != Difficulty::Hard
            || rand::random::<f64>()
                >= self.mob_entity.living_entity.get_attribute_value(
                    &pumpkin_data::attributes::Attributes::ZOMBIE_SPAWN_REINFORCEMENTS,
                )
        {
            return;
        }
        let Some(target) = self
            .mob_entity
            .target
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        else {
            return;
        };

        let pos = entity.pos.load();
        let (x, y, z) = (
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );
        // Vanilla `Mth.nextInt(random, 7, 40) * Mth.nextInt(random, -1, 1)`:
        // 7 to 40 blocks out, in a random cardinal direction per axis.
        let offset = || rand::random_range(7..40) * rand::random_range(-1..=1);

        for _ in 0..50 {
            let spawn_pos = pumpkin_util::math::position::BlockPos::new(
                x + offset(),
                y + offset(),
                z + offset(),
            );
            let world = entity.world.load_full();
            if !crate::world::natural_spawner::is_spawn_position_ok(
                &world,
                &spawn_pos,
                &EntityType::ZOMBIE,
            ) || !crate::entity::r#type::check_spawn_rules(
                &EntityType::ZOMBIE,
                &world,
                &spawn_pos,
                false,
            ) {
                continue;
            }
            let spawn_point = spawn_pos.to_centered_f64();
            if world.players.load().iter().any(|player| {
                player
                    .living_entity
                    .entity
                    .pos
                    .load()
                    .squared_distance_to_vec(&spawn_point)
                    < 49.0
            }) {
                continue;
            }

            let reinforcement = zombie::ZombieEntity::new(Entity::new(
                world.clone(),
                spawn_point,
                &EntityType::ZOMBIE,
            ));
            let reinforcement: Arc<dyn Mob> = reinforcement;
            reinforcement
                .get_mob_entity()
                .set_target(Some(target.clone()));
            reinforcement.finalize_spawn(&world, None);

            // Vanilla's caller/callee charge modifiers push the chance of
            // chain reactions down.
            let mut attributes = self
                .mob_entity
                .living_entity
                .attributes
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(reinforcements) = attributes
                .get_mut(&pumpkin_data::attributes::Attributes::ZOMBIE_SPAWN_REINFORCEMENTS.id)
            {
                let caller_charge = reinforcements
                    .modifiers
                    .iter()
                    .find(|modifier| modifier.id == "minecraft:reinforcement_caller_charge")
                    .map_or(0.0, |modifier| modifier.amount);
                reinforcements
                    .modifiers
                    .retain(|modifier| modifier.id != "minecraft:reinforcement_caller_charge");
                reinforcements.add_or_replace_modifier(crate::entity::attributes::Modifier {
                    id: "minecraft:reinforcement_caller_charge".to_string(),
                    amount: caller_charge - 0.05,
                    operation: crate::entity::attributes::ModifierOperation::Add,
                });
            }
            if let Some(reinforcements) = reinforcement
                .get_mob_entity()
                .living_entity
                .attributes
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get_mut(&pumpkin_data::attributes::Attributes::ZOMBIE_SPAWN_REINFORCEMENTS.id)
            {
                reinforcements.add_or_replace_modifier(crate::entity::attributes::Modifier {
                    id: "minecraft:reinforcement_callee_charge".to_string(),
                    amount: -0.05,
                    operation: crate::entity::attributes::ModifierOperation::Add,
                });
            }

            world.spawn_entity(reinforcement.clone());
            break;
        }
    }
}

impl Mob for ZombieEntityBase {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn spawn_as_baby(&self) -> bool {
        self.set_baby(true);
        true
    }

    fn populate_default_equipment_slots(
        &self,
        _world: &Arc<World>,
        difficulty: &RegionalDifficulty,
    ) {
        // Default armor slots (super.populateDefaultEquipmentSlots)
        if rand::random::<f32>()
            < MobEntity::MAX_WEARING_ARMOR_CHANCE * difficulty.special_multiplier
        {
            let mut armor_type = rand::random_range(0..3);
            for _ in 1..=3 {
                if rand::random::<f32>() < MobEntity::WEARING_ARMOR_UPGRADE_MATERIAL_CHANCE {
                    armor_type += 1;
                }
            }

            let partial_chance = if difficulty.base_difficulty == Difficulty::Hard {
                0.1f32
            } else {
                0.25f32
            };

            let living = &self.mob_entity.living_entity;
            let mut equipment = living
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut first = true;

            for slot in &MobEntity::EQUIPMENT_POPULATION_ORDER {
                let current = equipment.get(slot);
                if !first && rand::random::<f32>() < partial_chance {
                    break;
                }
                first = false;
                if current.is_empty()
                    && let Some(item) = MobEntity::get_equipment_for_slot(slot, armor_type)
                {
                    equipment.put(slot, ItemStack::new(1, item));
                }
            }
        }

        let weapon_chance = if difficulty.base_difficulty == Difficulty::Hard {
            0.05f32
        } else {
            0.01f32
        };
        if rand::random::<f32>() < weapon_chance {
            let r = rand::random_range(0..6);
            let weapon_item = match r {
                0 => &Item::IRON_SWORD,
                _ => &Item::IRON_SHOVEL,
            };
            let living = &self.mob_entity.living_entity;
            let mut equipment = living
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            equipment.put(&EquipmentSlot::MAIN_HAND, ItemStack::new(1, weapon_item));
        }
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        // Vanilla Zombie.addAdditionalSaveData; shared by every zombie-family mob.
        nbt.put_bool("IsBaby", self.is_baby());
        if self.can_break_doors() {
            nbt.put_bool("CanBreakDoors", true);
        }
        let in_water = self
            .mob_entity
            .living_entity
            .entity
            .touching_water
            .load(Ordering::Relaxed);
        nbt.put_int(
            "InWaterTime",
            if in_water {
                self.in_water_time.load(Ordering::Relaxed)
            } else {
                -1
            },
        );
        nbt.put_int(
            "DrownedConversionTime",
            if self.conversion_time.load(Ordering::Relaxed) >= 0 {
                self.conversion_time.load(Ordering::Relaxed)
            } else {
                -1
            },
        );
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        self.set_baby(nbt.get_bool("IsBaby").unwrap_or(false));
        if let Some(can_break_doors) = nbt.get_bool("CanBreakDoors") {
            self.set_can_break_doors(can_break_doors, self);
        }
        self.in_water_time
            .store(nbt.get_int("InWaterTime").unwrap_or(-1), Ordering::Relaxed);
        if let Some(conversion_time) = nbt.get_int("DrownedConversionTime")
            && conversion_time > -1
        {
            self.start_under_water_conversion(conversion_time);
        }
    }
}

/// The zombie-family hooks every wrapper delegates to its
/// [`ZombieEntityBase`]; `converts_in_water` is vanilla's
/// `Zombie.convertsInWater` (false for the drowned).
#[macro_export]
macro_rules! delegate_zombie_hooks {
    ($converts_in_water:expr) => {
        fn mob_tick(&self, caller: &dyn $crate::entity::EntityBase) {
            self.entity
                .tick_water_conversion(caller, $converts_in_water);
        }

        fn killed_entity(
            &self,
            world: &std::sync::Arc<$crate::world::World>,
            target: &std::sync::Arc<dyn $crate::entity::EntityBase>,
        ) -> bool {
            self.entity.killed_entity_villager(world, target)
        }

        fn after_melee_hit(&self, target: &dyn $crate::entity::EntityBase) {
            self.entity.after_melee_hit_burn(target);
        }

        fn on_damage(
            &self,
            _damage_type: pumpkin_data::damage::DamageType,
            _source: Option<&dyn $crate::entity::EntityBase>,
        ) {
            self.entity.on_damage_reinforcements();
        }

        fn as_zombie(&self) -> Option<&$crate::entity::mob::zombie::ZombieEntityBase> {
            Some(&self.entity)
        }
    };
}
