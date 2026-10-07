use std::sync::{Arc, Weak};

use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::item::Item;

use super::{Controls, Goal};
use crate::entity::EntityBase;
use crate::entity::ai::pathfinder::NavigatorGoal;
use crate::entity::mob::{Mob, RangedAttackMob};

/// Vanilla `DrownedEntity.DrownedAttackGoal`, the trident throw. Unlike the plain
/// ranged goal this one only ever runs while the trident is actually in hand, and
/// only against a target the drowned is allowed to hit (vanilla
/// `DrownedEntity.okTarget`): something in the water, or anything once night falls.
pub struct DrownedTridentAttackGoal {
    mob: Weak<dyn RangedAttackMob>,
    speed_modifier: f64,
    attack_interval: i32,
    attack_radius: f32,
    attack_radius_sqr: f64,
    see_time: i32,
    attack_time: i32,
    target: Option<Arc<dyn EntityBase>>,
}

impl DrownedTridentAttackGoal {
    #[must_use]
    pub const fn new(
        mob: Weak<dyn RangedAttackMob>,
        speed_modifier: f64,
        attack_interval: i32,
        attack_radius: f32,
    ) -> Self {
        Self {
            mob,
            speed_modifier,
            attack_interval,
            attack_radius,
            attack_radius_sqr: (attack_radius * attack_radius) as f64,
            see_time: 0,
            attack_time: 0,
            target: None,
        }
    }

    /// Vanilla drowneds attack with the trident they spawned holding, and melee
    /// (`ZombieAttackGoal`) covers the ones that spawned with something else.
    #[must_use]
    fn holds_trident(mob: &dyn Mob) -> bool {
        let equipment = mob
            .get_mob_entity()
            .living_entity
            .entity_equipment
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        equipment.get(&EquipmentSlot::MAIN_HAND).item.id == Item::TRIDENT.id
    }
}

impl Goal for DrownedTridentAttackGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        let target = mob.get_mob_entity().get_target().clone();
        if let Some(target) = target
            && target.get_entity().is_alive()
            && Self::holds_trident(mob)
        {
            self.target = Some(target);
            true
        } else {
            false
        }
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        let Some(target) = self.target.as_ref() else {
            return false;
        };

        target.get_entity().is_alive()
            && mob.get_mob_entity().get_target().is_some()
            && Self::holds_trident(mob)
    }

    fn start(&mut self, _mob: &dyn Mob) {
        // Vanilla waits out a full interval before the first throw.
        self.see_time = 0;
        self.attack_time = self.attack_interval;
    }

    fn stop(&mut self, mob: &dyn Mob) {
        self.target = None;
        self.see_time = 0;
        self.attack_time = 0;
        mob.get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .stop();
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn controls(&self) -> Controls {
        Controls::MOVE | Controls::LOOK
    }

    fn tick(&mut self, mob: &dyn Mob) {
        let Some(target) = self.target.clone() else {
            return;
        };
        let Some(ranged) = self.mob.upgrade() else {
            return;
        };

        let mob_pos = mob.get_entity().pos.load();
        let target_pos = target.get_entity().pos.load();
        let target_dist_sq = mob_pos.squared_distance_to_vec(&target_pos);

        let has_line_of_sight = mob.has_line_of_sight(target.get_entity());
        if has_line_of_sight {
            self.see_time += 1;
        } else {
            self.see_time = 0;
        }

        {
            let mut navigator = mob
                .get_mob_entity()
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if target_dist_sq <= self.attack_radius_sqr && self.see_time >= 5 {
                navigator.stop();
            } else {
                navigator.set_progress(NavigatorGoal {
                    current_progress: mob_pos,
                    destination: target_pos,
                    speed: self.speed_modifier,
                });
            }
        }

        mob.get_mob_entity()
            .look_control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .look_at_entity_with_range(&target, 30.0, 30.0);

        self.attack_time -= 1;
        if self.attack_time <= 0 {
            if has_line_of_sight {
                let distance_ratio = (target_dist_sq.sqrt() as f32) / self.attack_radius;
                ranged.perform_ranged_attack(&target, distance_ratio.clamp(0.1, 1.0));
            }
            self.attack_time = self.attack_interval;
        }
    }
}
