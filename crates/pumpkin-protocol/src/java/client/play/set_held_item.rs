use pumpkin_data::packet::clientbound::play::SET_CARRIED_ITEM;
use pumpkin_util::version::JavaMinecraftVersion;

use crate::ClientPacket;
use crate::packet::MultiVersionJavaPacket;
use crate::ser::NetworkWriteExt;

pub struct CSetSelectedSlot {
    pub slot: i8,
}

impl CSetSelectedSlot {
    #[must_use]
    pub const fn new(slot: i8) -> Self {
        Self { slot }
    }
}

impl ClientPacket for CSetSelectedSlot {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        let _ = version;
        write.write_i8(self.slot)?;
        Ok(())
    }
}

impl MultiVersionJavaPacket for CSetSelectedSlot {
    fn to_id(version: JavaMinecraftVersion) -> i32 {
        SET_CARRIED_ITEM.to_id(version)
    }
}
