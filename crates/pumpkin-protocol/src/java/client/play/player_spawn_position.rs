use std::io::Write;

use pumpkin_data::packet::clientbound::play::SET_DEFAULT_SPAWN_POSITION;
use pumpkin_macros::java_packet;
use pumpkin_util::{math::position::BlockPos, version::JavaMinecraftVersion};

use crate::{
    ClientPacket, ServerPacket,
    ser::{NetworkReadExt, NetworkWriteExt, ReadingError, WritingError},
};

/// Sent by the server to set the client's default spawn point and compass target.
///
/// This packet updates where the player will respawn upon death (if no bed or anchor is set)
/// and dictates the coordinates that a compass will point toward.
#[java_packet(SET_DEFAULT_SPAWN_POSITION)]
pub struct CPlayerSpawnPosition {
    /// The X, Y, and Z coordinates of the spawn location.
    pub location: BlockPos,
    /// The horizontal rotation (0-360 degrees) the player's camera should face upon respawning.
    /// (1.17+)
    pub yaw: f32,
}

impl CPlayerSpawnPosition {
    #[must_use]
    pub const fn new(location: BlockPos, yaw: f32) -> Self {
        Self { location, yaw }
    }
}

impl ClientPacket for CPlayerSpawnPosition {
    fn write_packet_data(
        &self,
        mut write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_8 {
            write.write_block_pos(&self.location, version)?;
        } else {
            write.write_i32_be(self.location.0.x)?;
            write.write_i32_be(self.location.0.y)?;
            write.write_i32_be(self.location.0.z)?;
        }

        if *version >= JavaMinecraftVersion::V_1_17 {
            write.write_f32_be(self.yaw)?;
        }

        Ok(())
    }
}

impl<'a> ServerPacket<'a> for CPlayerSpawnPosition {
    fn read(read: &mut &'a [u8], version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        let location = read.get_block_pos(version)?;

        let yaw = if *version >= JavaMinecraftVersion::V_1_17 {
            read.get_f32_be()?
        } else {
            0.0
        };

        Ok(Self { location, yaw })
    }
}
