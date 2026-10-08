use pumpkin_data::packet::serverbound::play::PICK_ITEM;
use pumpkin_macros::java_packet;

use crate::{ServerPacket, ser::ReadingError};
use pumpkin_util::version::JavaMinecraftVersion;

/// Requests that the server put the picked block or entity into the player's
/// hotbar. The packet carries no payload.
#[java_packet(PICK_ITEM)]
pub struct SPickItem;

impl<'a> ServerPacket<'a> for SPickItem {
    fn read(
        _bytebuf: &mut &'a [u8],
        _version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError> {
        Ok(Self)
    }
}

impl crate::ClientPacket for SPickItem {
    fn write_packet_data(
        &self,
        _write: impl std::io::Write,
        _version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        Ok(())
    }
}
