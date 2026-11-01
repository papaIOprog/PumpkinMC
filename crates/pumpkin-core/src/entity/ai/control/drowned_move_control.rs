use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use pumpkin_data::attributes::Attributes;
use pumpkin_util::math::vector3::Vector3;

use crate::entity::ai::control::move_control::MoveControl;
use crate::entity::ai::control::{Control, MoveControlTrait};
use crate::entity::mob::Mob;

/// Vanilla `Drowned.DrownedMoveControl`.
///
/// A drowned steers itself through water instead of walking: aiming its whole
/// velocity at the point it is swimming to, so it rises, sinks and turns towards a
/// target that a land mob's move control could never reach. It only borrows the land
/// controller's handling while it is out of the water, or in water without a reason
/// to swim, where it sinks so it settles on the bottom.
///
/// Vanilla reads the point and the speed off its own state, because the navigation
/// there calls `setWantedPosition` on the move control every tick. Nothing in this
/// port ever does — the navigation aims the mob and writes the movement input itself —
/// so this reads the navigator's target instead, which is the same point by another
/// name. Depending on `set_wanted_position` here would leave the swim target at the
/// origin, and the speed at zero.
#[derive(Default)]
pub struct DrownedMoveControl {
    /// The walking controller, for the vanilla `super.tick()` path.
    inner: MoveControl,
}

impl Control for DrownedMoveControl {}

impl DrownedMoveControl {
    /// How close two coordinates have to be before `atan2` has no direction left to
    /// report: `atan2(0, 0)` is zero, which is due east wherever the target really is.
    const NO_DIRECTION: f64 = 1.0e-6;

    fn swim_target(mob: &dyn Mob) -> Option<Vector3<f64>> {
        let mob_entity = mob.get_mob_entity();
        let navigator = mob_entity
            .navigator
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        // Vanilla's navigation feeds the move control the waypoint it is on
        // (`setWantedPosition` each tick); steer at the current path node.
        let waypoint = navigator
            .get_path()
            .and_then(|path| path.get_next_node_pos())
            .map(|pos| {
                Vector3::new(
                    f64::from(pos.0.x) + 0.5,
                    f64::from(pos.0.y) + 0.5,
                    f64::from(pos.0.z) + 0.5,
                )
            });

        // A drowned whose path ran out still swims at what it is chasing.
        waypoint.or_else(|| {
            mob_entity
                .target
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_ref()
                .map(|target| target.get_entity().pos.load())
        })
    }
}

impl MoveControlTrait for DrownedMoveControl {
    fn tick(&mut self, mob: &dyn Mob) {
        let mob_entity = mob.get_mob_entity();
        let living = &mob_entity.living_entity;
        let entity = &living.entity;

        if !(mob.wants_to_swim() && entity.touching_water.load(Ordering::Relaxed)) {
            // Vanilla's else branch: drift down when airborne, then walk as usual.
            if !entity.on_ground.load(Ordering::Relaxed) {
                let velocity = entity.velocity.load();
                entity
                    .velocity
                    .store(Vector3::new(velocity.x, velocity.y - 0.008, velocity.z));
            }

            // The walking controller now swims land mobs at their waypoint. A drowned
            // that has decided not to swim — a land target in daylight, say — has to
            // sink instead of being steered after it, so it keeps the water to itself.
            if !entity.touching_water.load(Ordering::Relaxed) {
                self.inner.tick(mob);
            }
            return;
        }

        let Some(wanted) = Self::swim_target(mob) else {
            living.movement_input.store(Vector3::new(0.0, 0.0, 0.0));
            return;
        };

        let pos = entity.pos.load();
        let xd = wanted.x - pos.x;
        let mut yd = wanted.y - pos.y;
        let zd = wanted.z - pos.z;

        // Rise slowly while it is swimming up at something, or looking for a shore to
        // climb out on.
        if yd > 0.0 || mob.is_searching_for_land() {
            let velocity = entity.velocity.load();
            entity
                .velocity
                .store(Vector3::new(velocity.x, velocity.y + 0.002, velocity.z));
        }

        let distance = (xd * xd + yd * yd + zd * zd).sqrt();
        if distance < 1.0e-5 {
            living.movement_input.store(Vector3::new(0.0, 0.0, 0.0));
            return;
        }

        // Vanilla divides only the vertical by the full distance, which is what makes
        // the climb gentle while the horizontal stays at full pace.
        yd /= distance;

        // Only aim when there is a horizontal direction to aim at. A swim target
        // straight above or below gives none, and the yaw `atan2` returns for that is
        // arbitrary — facing due east while the drowned is really swimming up at a
        // player behind it.
        if xd * xd + zd * zd > Self::NO_DIRECTION {
            let yaw = self.change_angle(
                entity.yaw.load(),
                (zd.atan2(xd).to_degrees() as f32) - 90.0,
                90.0,
            );
            entity.yaw.store(pumpkin_util::math::wrap_degrees(yaw));
            entity.body_yaw.store(yaw);
        }

        // The navigator put the swim speed in the movement input this tick; vanilla
        // reads the same number off its own state. Leave the input alone — it is what
        // the water travel turns into motion — and push the velocity the same way.
        let speed = living.movement_input.load().z;
        let speed = if speed.abs() < 1.0e-6 {
            living.get_attribute_value(&Attributes::GENERIC_MOVEMENT_SPEED)
        } else {
            speed
        };

        let velocity = entity.velocity.load();
        entity.velocity.store(Vector3::new(
            velocity.x + speed * xd * 0.005,
            velocity.y + speed * yd * 0.1,
            velocity.z + speed * zd * 0.005,
        ));
    }

    fn set_wanted_position(&mut self, x: f64, y: f64, z: f64, speed_modifier: f64) {
        self.inner.set_wanted_position(x, y, z, speed_modifier);
    }

    fn strafe(&mut self, forward: f32, right: f32) {
        self.inner.strafe(forward, right);
    }

    fn has_wanted(&self) -> bool {
        self.inner.has_wanted()
    }
}
