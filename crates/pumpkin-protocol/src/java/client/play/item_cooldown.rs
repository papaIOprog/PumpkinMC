use pumpkin_data::packet::clientbound::play::COOLDOWN;

use crate::codec::var_int::VarInt;
use crate::ser::NetworkReadExt;
use crate::ser::NetworkWriteExt;
use crate::{ClientPacket, ReadingError, ServerPacket};
use pumpkin_macros::java_packet;
use pumpkin_util::version::JavaMinecraftVersion;

/// Vanilla 1.21.1 `ClientboundCooldownPacket`: the item's registry id and the
/// cooldown in ticks (0 ends it). The string-group form is 1.21.2+.
#[java_packet(COOLDOWN)]
pub struct CItemCooldown {
    pub item_id: VarInt,
    pub cooldown: VarInt,
}

impl CItemCooldown {
    #[must_use]
    pub const fn new(item_id: VarInt, cooldown: VarInt) -> Self {
        Self { item_id, cooldown }
    }
}

impl ClientPacket for CItemCooldown {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        _version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        write.write_var_int(&self.item_id)?;
        write.write_var_int(&self.cooldown)?;
        Ok(())
    }
}

impl<'a> ServerPacket<'a> for CItemCooldown {
    fn read(bytebuf: &mut &'a [u8], _version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        Ok(Self {
            item_id: bytebuf.get_var_int()?,
            cooldown: bytebuf.get_var_int()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ser::NetworkReadExt;
    use pumpkin_util::version::JavaMinecraftVersion;

    #[test]
    fn round_trips_item_id_and_ticks() {
        let mut bytes = Vec::new();
        CItemCooldown::new(VarInt(1234), VarInt(20))
            .write_packet_data(&mut bytes, &JavaMinecraftVersion::V_1_21)
            .unwrap();
        // Two varints, nothing else: 1234 = 0xD2 0x09, 20 = 0x14.
        assert_eq!(bytes, [0xD2, 0x09, 0x14]);
        let mut cursor: &[u8] = &bytes;
        let read = CItemCooldown::read(&mut cursor, &JavaMinecraftVersion::V_1_21).unwrap();
        assert_eq!(read.item_id.0, 1234);
        assert_eq!(read.cooldown.0, 20);
    }
}
