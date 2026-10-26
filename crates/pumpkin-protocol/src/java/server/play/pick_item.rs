use pumpkin_data::packet::serverbound::play::PICK_ITEM;
use pumpkin_macros::java_packet;

use crate::ser::{NetworkReadExt, NetworkWriteExt};
use crate::{ServerPacket, VarInt, ser::ReadingError};
use pumpkin_util::version::JavaMinecraftVersion;

/// Requests that the server put the picked block or entity into the player's
/// hotbar.
#[java_packet(PICK_ITEM)]
pub struct SPickItem {
    /// The slot to pick the item from.
    pub slot_to_use: VarInt,
}

impl<'a> ServerPacket<'a> for SPickItem {
    fn read(bytebuf: &mut &'a [u8], _version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        let slot_to_use = bytebuf.get_var_int()?;
        Ok(Self { slot_to_use })
    }
}

impl crate::ClientPacket for SPickItem {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        _version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        write.write_var_int(&self.slot_to_use)?;
        Ok(())
    }
}
