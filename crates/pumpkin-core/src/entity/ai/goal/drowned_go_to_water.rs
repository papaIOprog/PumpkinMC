use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use super::{Controls, Goal};
use crate::entity::ai::util::goal_utils;
use crate::entity::mob::Mob;

/// Vanilla `Drowned.DrownedGoToWaterGoal`.
///
/// The daytime half of a drowned's day/night split. While the sun is up a drowned
/// refuses to stay on dry land, so it draws random blocks around itself, takes the
/// first that is water, and walks to it. Vanilla draws the spot rather than searching
/// from it, so it will happily pick water it cannot reach and simply not arrive.
pub struct DrownedGoToWaterGoal {
    speed: f64,
    wanted: Option<Vector3<f64>>,
}

impl DrownedGoToWaterGoal {
    #[must_use]
    pub fn new(speed: f64) -> Box<Self> {
        Box::new(Self {
            speed,
            wanted: None,
        })
    }

    /// Vanilla `getWaterPos`: ten draws of a block up to ten out, two up and seven
    /// down, taking the first that is water.
    fn water_pos(mob: &dyn Mob) -> Option<Vector3<f64>> {
        let world = mob.get_entity().world.load_full();
        let origin = mob.get_entity().block_pos.load().0;
        let mut rng = mob.get_random();

        for _ in 0..10 {
            let x = origin.x + rng.random_range(-10..=10);
            let y = origin.y + 2 - rng.random_range(0..=7);
            let z = origin.z + rng.random_range(-10..=10);

            if goal_utils::is_water(&world, &BlockPos::new(x, y, z)) {
                return Some(Vector3::new(
                    f64::from(x) + 0.5,
                    f64::from(y),
                    f64::from(z) + 0.5,
                ));
            }
        }

        None
    }
}

impl Goal for DrownedGoToWaterGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        // Vanilla `!level.isBrightOutside()`: daylight only, which is what keeps a
        // drowned in its pond while the sun is out.
        if !mob.get_entity().world.load().is_day() {
            return false;
        }

        // Already wet: nothing to walk to.
        if mob.get_entity().touching_water.load(Ordering::Relaxed) {
            return false;
        }

        self.wanted = Self::water_pos(mob);
        self.wanted.is_some()
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        !mob.is_navigator_idle()
    }

    fn start(&mut self, mob: &dyn Mob) {
        let Some(wanted) = self.wanted else {
            return;
        };

        mob.get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .move_to_coords(
                wanted.x,
                wanted.y,
                wanted.z,
                self.speed,
                &mob.get_mob_entity().living_entity,
            );
    }

    fn controls(&self) -> Controls {
        Controls::MOVE
    }
}
