use crate::{
    ServerPacket,
    ser::{NetworkReadExt, NetworkReadSliceExt, ReadingError},
};
use pumpkin_data::packet::serverbound::play::PLACE_RECIPE;
use pumpkin_macros::java_packet;
use pumpkin_util::version::JavaMinecraftVersion;

#[java_packet(PLACE_RECIPE)]
pub struct SPlaceRecipe {
    pub container_id: i8,
    /// The recipe's string key, e.g. `minecraft:oak_planks`.
    pub recipe_id: String,
    pub use_max_items: bool,
}

impl<'a> ServerPacket<'a> for SPlaceRecipe {
    fn read(bytebuf: &mut &'a [u8], _version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        let container_id = bytebuf.get_i8()?;
        let recipe_id = bytebuf.get_str_borrowed()?.to_string();
        let use_max_items = bytebuf.get_bool()?;
        Ok(Self {
            container_id,
            recipe_id,
            use_max_items,
        })
    }
}

impl crate::ClientPacket for SPlaceRecipe {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        _version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        use crate::ser::NetworkWriteExt;
        write.write_i8(self.container_id)?;
        write.write_string(&self.recipe_id)?;
        write.write_bool(self.use_max_items)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientPacket;
    use pumpkin_util::version::JavaMinecraftVersion;

    #[test]
    fn round_trips_the_recipe_key() {
        let mut bytes = Vec::new();
        SPlaceRecipe {
            container_id: 3,
            recipe_id: "minecraft:oak_planks".to_string(),
            use_max_items: true,
        }
        .write_packet_data(&mut bytes, &JavaMinecraftVersion::V_1_21)
        .unwrap();

        let mut cursor: &[u8] = &bytes;
        let read = SPlaceRecipe::read(&mut cursor, &JavaMinecraftVersion::V_1_21).unwrap();
        assert_eq!(read.container_id, 3);
        assert_eq!(read.recipe_id, "minecraft:oak_planks");
        assert!(read.use_max_items);
        assert!(cursor.is_empty());
    }
}
