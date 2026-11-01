use crate::entity::ai::control::{Control, MoveControlTrait};
use crate::entity::mob::Mob;
use pumpkin_data::attributes::Attributes;
use pumpkin_util::math::vector3::Vector3;
use std::sync::PoisonError;
use std::sync::atomic::Ordering;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    #[default]
    Wait,
    MoveTo,
    Strafe,
    Jumping,
}

pub struct MoveControl {
    pub wanted_x: f64,
    pub wanted_y: f64,
    pub wanted_z: f64,
    pub speed_modifier: f64,
    pub strafe_forwards: f32,
    pub strafe_right: f32,
    pub operation: Operation,
}

impl Default for MoveControl {
    fn default() -> Self {
        Self {
            wanted_x: 0.0,
            wanted_y: 0.0,
            wanted_z: 0.0,
            speed_modifier: 0.0,
            strafe_forwards: 0.0,
            strafe_right: 0.0,
            operation: Operation::Wait,
        }
    }
}

impl Control for MoveControl {}

impl MoveControlTrait for MoveControl {
    fn tick(&mut self, mob: &dyn Mob) {
        let mob_entity = mob.get_mob_entity();
        let living_entity = &mob_entity.living_entity;
        let entity = &living_entity.entity;

        // A land mob in water swims the way a drowned does: aim its whole velocity at
        // the point it is heading for. The navigator only turns it towards path nodes,
        // which is not enough once it is floating and has no ground to push off. An
        // explicit request from a goal (MoveTo/Strafe) still wins.
        if matches!(self.operation, Operation::Wait | Operation::Jumping)
            && entity.touching_water.load(Ordering::Relaxed)
            && entity.water_height.load() > living_entity.get_swim_height()
            && self.swim_towards(mob, false)
        {
            return;
        }

        if self.operation == Operation::Strafe {
            // TODO: is_walkable check
            living_entity.movement_input.store(Vector3::new(
                self.strafe_right as f64,
                0.0,
                self.strafe_forwards as f64,
            ));
            // Vanilla sets speed here too
            self.operation = Operation::Wait;
        } else if self.operation == Operation::MoveTo {
            self.operation = Operation::Wait;
            let pos = entity.pos.load();
            let xd = self.wanted_x - pos.x;
            let zd = self.wanted_z - pos.z;
            let yd = self.wanted_y - pos.y;
            let dd = xd * xd + yd * yd + zd * zd;

            if dd < 2.5000003E-7 {
                living_entity
                    .movement_input
                    .store(Vector3::new(0.0, 0.0, 0.0));
                return;
            }

            let y_rot_d = (zd.atan2(xd).to_degrees() as f32) - 90.0;
            let yaw = self.change_angle(entity.yaw.load(), y_rot_d, 90.0);
            entity.yaw.store(pumpkin_util::math::wrap_degrees(yaw));

            let movement_speed =
                living_entity.get_attribute_value(&Attributes::GENERIC_MOVEMENT_SPEED);
            let speed = self.speed_modifier * movement_speed;
            living_entity
                .movement_input
                .store(Vector3::new(0.0, 0.0, speed));

            // TODO: Jump if needed (based on collision and height difference)
            let step_height = living_entity.get_attribute_value(&Attributes::GENERIC_STEP_HEIGHT);
            if yd > step_height
                && xd * xd + zd * zd < 1.0f64.max(entity.entity_dimension.load().width as f64)
            {
                living_entity.jumping.store(true, Ordering::SeqCst);
                self.operation = Operation::Jumping;
            }
        } else if self.operation == Operation::Jumping {
            let movement_speed =
                living_entity.get_attribute_value(&Attributes::GENERIC_MOVEMENT_SPEED);
            let speed = self.speed_modifier * movement_speed;
            living_entity
                .movement_input
                .store(Vector3::new(0.0, 0.0, speed));

            if entity.on_ground.load(Ordering::Relaxed) {
                self.operation = Operation::Wait;
            }
        }

        // Navigator owns movement input while this controller waits.
    }

    fn set_wanted_position(&mut self, x: f64, y: f64, z: f64, speed_modifier: f64) {
        self.wanted_x = x;
        self.wanted_y = y;
        self.wanted_z = z;
        self.speed_modifier = speed_modifier;
        if self.operation != Operation::Jumping {
            self.operation = Operation::MoveTo;
        }
    }

    fn strafe(&mut self, forwards: f32, right: f32) {
        self.operation = Operation::Strafe;
        self.strafe_forwards = forwards;
        self.strafe_right = right;
        self.speed_modifier = 0.25;
    }

    fn has_wanted(&self) -> bool {
        self.operation == Operation::MoveTo
    }
}

impl MoveControl {
    #[must_use]
    pub fn has_wanted(&self) -> bool {
        self.operation == Operation::MoveTo
    }

    #[must_use]
    pub const fn get_speed_modifier(&self) -> f64 {
        self.speed_modifier
    }

    /// How close two coordinates have to be before `atan2` has no direction left to
    /// report: `atan2(0, 0)` is zero, which is due east wherever the target really is.
    const NO_DIRECTION: f64 = 1.0e-6;

    /// The point a swimming mob steers at: where the navigator is taking it, or failing
    /// that whatever it is chasing.
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
            .and_then(crate::entity::ai::pathfinder::path::Path::get_next_node_pos)
            .map(|pos| {
                Vector3::new(
                    f64::from(pos.0.x) + 0.5,
                    f64::from(pos.0.y) + 0.5,
                    f64::from(pos.0.z) + 0.5,
                )
            });

        // A mob whose path ran out still swims at what it is chasing.
        waypoint.or_else(|| {
            mob_entity
                .target
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_ref()
                .map(|target| target.get_entity().pos.load())
        })
    }

    /// Swim at the waypoint the way vanilla's `Drowned.DrownedMoveControl` does, which
    /// divides only the vertical by the distance so the climb stays gentle while the
    /// horizontal keeps full pace.
    ///
    /// Returns whether it steered. `force_rise` is vanilla's `searchingForLand`, which
    /// keeps a mob climbing while it looks for a shore to climb out on.
    fn swim_towards(&self, mob: &dyn Mob, force_rise: bool) -> bool {
        let mob_entity = mob.get_mob_entity();
        let living_entity = &mob_entity.living_entity;
        let entity = &living_entity.entity;

        let Some(wanted) = Self::swim_target(mob) else {
            return false;
        };

        let pos = entity.pos.load();
        let xd = wanted.x - pos.x;
        let mut yd = wanted.y - pos.y;
        let zd = wanted.z - pos.z;

        // Rise slowly while it is swimming up at something.
        if yd > 0.0 || force_rise {
            let velocity = entity.velocity.load();
            entity
                .velocity
                .store(Vector3::new(velocity.x, velocity.y + 0.002, velocity.z));
        }

        let distance = (xd * xd + yd * yd + zd * zd).sqrt();
        if distance < 1.0e-5 {
            living_entity
                .movement_input
                .store(Vector3::new(0.0, 0.0, 0.0));
            return true;
        }

        yd /= distance;

        // Only aim when there is a horizontal direction to aim at. A swim target
        // straight above or below gives none, and the yaw `atan2` returns for that is
        // arbitrary — facing due east while really swimming up at something behind it.
        if xd * xd + zd * zd > Self::NO_DIRECTION {
            let yaw = self.change_angle(
                entity.yaw.load(),
                (zd.atan2(xd).to_degrees() as f32) - 90.0,
                90.0,
            );
            entity.yaw.store(pumpkin_util::math::wrap_degrees(yaw));
            entity.body_yaw.store(yaw);
        }

        // The navigator put the swim speed in the movement input this tick; leave it —
        // it is what the water travel turns into motion — and push the velocity.
        let speed = living_entity.movement_input.load().z;
        let speed = if speed.abs() < 1.0e-6 {
            living_entity.get_attribute_value(&Attributes::GENERIC_MOVEMENT_SPEED)
        } else {
            speed
        };

        let velocity = entity.velocity.load();
        entity.velocity.store(Vector3::new(
            velocity.x + speed * xd * 0.005,
            velocity.y + speed * yd * 0.1,
            velocity.z + speed * zd * 0.005,
        ));

        true
    }

    pub fn set_wanted_position(&mut self, x: f64, y: f64, z: f64, speed_modifier: f64) {
        self.wanted_x = x;
        self.wanted_y = y;
        self.wanted_z = z;
        self.speed_modifier = speed_modifier;
        if self.operation != Operation::Jumping {
            self.operation = Operation::MoveTo;
        }
    }

    pub const fn strafe(&mut self, forwards: f32, right: f32) {
        self.operation = Operation::Strafe;
        self.strafe_forwards = forwards;
        self.strafe_right = right;
        self.speed_modifier = 0.25;
    }
}
