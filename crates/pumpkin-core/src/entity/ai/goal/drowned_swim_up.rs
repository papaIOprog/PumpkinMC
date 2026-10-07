use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use pumpkin_util::math::vector3::Vector3;

use super::{Controls, Goal};
use crate::entity::ai::util::default_random_pos;
use crate::entity::mob::Mob;

/// Vanilla `Drowned.DrownedSwimUpGoal`.
///
/// After dark a drowned that is sitting deep tries to rise towards sea level, drawing
/// one spot at a time a few blocks up and out and swimming to it. Vanilla gives this
/// goal no controls at all, so it runs alongside whatever else the drowned is doing
/// instead of fighting it for movement.
pub struct DrownedSwimUpGoal {
    speed: f64,
    stuck: bool,
}

impl DrownedSwimUpGoal {
    /// Vanilla takes the level's sea level; this port fixes it at 63, as
    /// `PathfindingContext::sea_level` does.
    pub const SEA_LEVEL: i32 = 63;
    /// Vanilla `DefaultRandomPos.getPosTowards(..., (float)(Math.PI / 2))`.
    const SPREAD_RADIANS: f64 = std::f64::consts::FRAC_PI_2;

    #[must_use]
    pub fn new(speed: f64) -> Box<Self> {
        Box::new(Self {
            speed,
            stuck: false,
        })
    }

    /// Vanilla `canUse`: dark outside, in the water, and more than two blocks under
    /// the surface.
    fn wants_to_rise(mob: &dyn Mob) -> bool {
        let entity = mob.get_entity();

        !entity.world.load().is_day()
            && entity.touching_water.load(Ordering::Relaxed)
            && entity.pos.load().y < f64::from(Self::SEA_LEVEL - 2)
    }

    /// Vanilla `closeToNextPos`: within two blocks of the block the path is headed for.
    fn close_to_next_pos(mob: &dyn Mob) -> bool {
        let target = mob
            .get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_target_pos();

        let Some(target) = target else {
            return false;
        };

        let pos = mob.get_entity().pos.load();
        let dx = f64::from(target.0.x) - pos.x;
        let dy = f64::from(target.0.y) - pos.y;
        let dz = f64::from(target.0.z) - pos.z;

        dx * dx + dy * dy + dz * dz < 4.0
    }
}

impl Goal for DrownedSwimUpGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        Self::wants_to_rise(mob)
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        Self::wants_to_rise(mob) && !self.stuck
    }

    fn start(&mut self, mob: &dyn Mob) {
        // Vanilla turns this on for the whole climb: the move control reads it to keep
        // a drowned rising even with nothing in particular to chase.
        mob.set_searching_for_land(true);
        self.stuck = false;
    }

    fn stop(&mut self, mob: &dyn Mob) {
        mob.set_searching_for_land(false);
    }

    fn tick(&mut self, mob: &dyn Mob) {
        let pos = mob.get_entity().pos.load();

        // Vanilla only steers while it is still below sea level minus one, and only
        // when the last swim is finished or nearly so.
        if pos.y >= f64::from(Self::SEA_LEVEL - 1)
            || !(mob.is_navigator_idle() || Self::close_to_next_pos(mob))
        {
            return;
        }

        let towards = Vector3::new(pos.x, f64::from(Self::SEA_LEVEL - 1), pos.z);
        let Some(next) =
            default_random_pos::get_pos_towards(mob, 4, 8, towards, Self::SPREAD_RADIANS)
        else {
            // Vanilla gives up for good rather than retrying every tick.
            self.stuck = true;
            return;
        };

        mob.get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .move_to_coords(
                next.x,
                next.y,
                next.z,
                self.speed,
                &mob.get_mob_entity().living_entity,
            );
    }

    /// Vanilla sets no flags on this goal, so it contends with nothing.
    fn controls(&self) -> Controls {
        Controls::empty()
    }
}
