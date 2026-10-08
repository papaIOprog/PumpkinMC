use std::io::Write;

use pumpkin_data::packet::clientbound::play::SET_ENTITY_MOTION;
use pumpkin_macros::java_packet;
use pumpkin_util::{math::vector3::Vector3, version::JavaMinecraftVersion};

use crate::{
    ClientPacket, VarInt, WritingError, codec::lp_vector_3d::LpVector3d, ser::NetworkWriteExt,
};

/// Updates the velocity of an entity.
///
/// This packet informs the client of a sudden change in an entity's movement,
/// such as knockback from an attack, explosions, or being launched by a piston.
#[java_packet(SET_ENTITY_MOTION)]
pub struct CEntityVelocity {
    /// The Entity ID of the entity whose velocity is being set
    pub entity_id: VarInt,
    /// The velocity vector
    pub velocity: LpVector3d,
}

impl CEntityVelocity {
    #[must_use]
    pub const fn new(entity_id: VarInt, velocity: Vector3<f64>) -> Self {
        Self {
            entity_id,
            velocity: LpVector3d(velocity),
        }
    }
}

impl ClientPacket for CEntityVelocity {
    fn write_packet_data(
        &self,
        write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        let mut write = write;

        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i32_be(self.entity_id.0)?;
        } else {
            write.write_var_int(&self.entity_id)?;
        }

        let _ = version;
        self.velocity.write_legacy(&mut write)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pumpkin_util::version::JavaMinecraftVersion;

    use super::CEntityVelocity;
    use crate::{ClientPacket, VarInt, codec::lp_vector_3d::encode_legacy_velocity_component};

    fn encode_packet(version: JavaMinecraftVersion) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let packet = CEntityVelocity::new(
            VarInt(1),
            pumpkin_util::math::vector3::Vector3::new(0.5, -0.5, 0.25),
        );
        let mut out = Vec::new();
        packet.write_packet_data(&mut out, &version)?;
        Ok(out)
    }

    fn legacy_bytes(x: f64, y: f64, z: f64) -> Vec<u8> {
        let mut out = Vec::with_capacity(6);
        out.extend_from_slice(&encode_legacy_velocity_component(x).to_be_bytes());
        out.extend_from_slice(&encode_legacy_velocity_component(y).to_be_bytes());
        out.extend_from_slice(&encode_legacy_velocity_component(z).to_be_bytes());
        out
    }

    #[test]
    fn entity_velocity_uses_legacy_format() -> Result<(), Box<dyn std::error::Error>> {
        let encoded = encode_packet(JavaMinecraftVersion::V_1_21)?;
        let expected_legacy_tail = legacy_bytes(0.5, -0.5, 0.25);

        assert_eq!(encoded, [&[1], expected_legacy_tail.as_slice()].concat());
        Ok(())
    }
}
