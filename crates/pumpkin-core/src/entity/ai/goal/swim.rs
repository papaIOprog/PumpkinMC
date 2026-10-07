use std::sync::atomic::Ordering;

use super::{Controls, Goal};
use crate::entity::mob::Mob;
use rand::RngExt;

pub struct SwimGoal {
    goal_control: Controls,
}

impl Default for SwimGoal {
    fn default() -> Self {
        Self {
            // Deliberately no controls. Floating must not compete for the jump control:
            // any goal that claims jump (the attack and move-to-target goals do) stops
            // whatever holds it, which would switch a mob's buoyancy off mid-chase.
            goal_control: Controls::empty(),
        }
    }
}

impl SwimGoal {
    fn is_in_fluid(mob: &dyn Mob) -> bool {
        let living = &mob.get_mob_entity().living_entity;
        let entity = &living.entity;
        let in_water = entity.touching_water.load(Ordering::SeqCst)
            && entity.water_height.load() > living.get_swim_height();
        in_water || entity.touching_lava.load(Ordering::SeqCst)
    }
}

impl Goal for SwimGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        mob.get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_can_float(true);

        Self::is_in_fluid(mob)
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        Self::is_in_fluid(mob)
    }

    fn tick(&mut self, mob: &dyn Mob) {
        // While it is chasing something, height belongs to the swim controller: buoyancy
        // would fight a mob following a player who dived, and pumping it up is what used
        // to surface an attacking mob that was meant to stay under.
        if mob
            .get_mob_entity()
            .target
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
        {
            return;
        }

        // Push straight onto the velocity instead of setting the shared jump flag: the
        // navigator rewrites that flag every tick it follows a path, so a mob chasing
        // something would lose its buoyancy and sink. The 0.8 mirrors vanilla's jitter.
        if mob.get_random().random::<f32>() < 0.8 {
            let entity = &mob.get_mob_entity().living_entity.entity;
            let mut velocity = entity.velocity.load();
            velocity.y += 0.04;
            entity.velocity.store(velocity);
        }
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn controls(&self) -> Controls {
        self.goal_control
    }
}
