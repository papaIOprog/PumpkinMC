use std::sync::Arc;
use std::sync::atomic::Ordering;

use pumpkin_util::math::position::BlockPos;

use super::move_to_target_pos::{MoveToTargetPos, MoveToTargetPosGoal};
use super::{Controls, Goal, ParentHandle};
use crate::entity::mob::Mob;
use crate::world::World;

/// Vanilla `Drowned.DrownedGoToBeachGoal`: the night half of the split, and the
/// only time a drowned climbs out on purpose.
///
/// It looks for a shore block near the surface — two blocks of air over something
/// it can stand on — and walks to it.
pub struct DrownedGoToBeachGoal {
    move_to_target_pos_goal: MoveToTargetPosGoal<Self>,
}

impl DrownedGoToBeachGoal {
    /// Vanilla takes the level's sea level; this port fixes it at 63, as
    /// `PathfindingContext::sea_level` does.
    const SEA_LEVEL: i32 = 63;

    #[must_use]
    pub fn new(speed: f64) -> Box<Self> {
        // Vanilla `MoveToBlockGoal(this, speed, 8, 2)`.
        let mut this = Box::new(Self {
            move_to_target_pos_goal: MoveToTargetPosGoal::new(ParentHandle::none(), speed, 8, 2),
        });

        // SAFETY: `this` heap allocation address is pinned in Box and outlives `ParentHandle` references.
        this.move_to_target_pos_goal.move_to_target_pos = unsafe { ParentHandle::new(&this) };

        this
    }

    /// Vanilla cut off: a drowned deep down has no shore within reach.
    fn near_surface(y: f64) -> bool {
        y >= f64::from(Self::SEA_LEVEL - 3)
    }
}

impl Goal for DrownedGoToBeachGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        let entity = mob.get_entity();

        // Vanilla: dark outside, in the water, and no more than three blocks under
        // sea level.
        if entity.world.load().is_day()
            || !entity.touching_water.load(Ordering::Relaxed)
            || !Self::near_surface(entity.pos.load().y)
        {
            return false;
        }

        self.move_to_target_pos_goal.can_start(mob)
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        self.move_to_target_pos_goal.should_continue(mob)
    }

    fn start(&mut self, mob: &dyn Mob) {
        // Vanilla clears the flag here, so hunting for a shore is not mistaken for
        // swimming at something.
        mob.set_searching_for_land(false);
        self.move_to_target_pos_goal.start(mob);
    }

    fn stop(&mut self, mob: &dyn Mob) {
        self.move_to_target_pos_goal.stop(mob);
    }

    fn tick(&mut self, mob: &dyn Mob) {
        self.move_to_target_pos_goal.tick(mob);
    }

    fn should_run_every_tick(&self) -> bool {
        self.move_to_target_pos_goal.should_run_every_tick()
    }

    fn controls(&self) -> Controls {
        self.move_to_target_pos_goal.controls()
    }
}

impl MoveToTargetPos for DrownedGoToBeachGoal {
    /// Vanilla `isValidTarget`: two empty blocks above something a drowned can stand
    /// on, which is what a beach looks like.
    fn is_target_pos(&self, world: Arc<World>, block_pos: BlockPos) -> bool {
        let above = block_pos.up();
        let above_above = above.up();

        world.get_block_state(&above).is_air()
            && world.get_block_state(&above_above).is_air()
            && world.get_block_state(&block_pos).is_solid()
    }
}
