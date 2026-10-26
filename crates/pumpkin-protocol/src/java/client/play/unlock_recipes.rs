use std::io::Write;

use pumpkin_util::identifier::Identifier;

use crate::VarInt;
use crate::{
    ClientPacket, ReadingError, ServerPacket, WritingError,
    ser::{NetworkReadExt, NetworkWriteExt},
};

use pumpkin_data::packet::clientbound::play::UNLOCK_RECIPES;
use pumpkin_macros::java_packet;
use pumpkin_util::version::JavaMinecraftVersion;

pub const RECIPE_BOOK_STATE_INIT: i32 = 0;
pub const RECIPE_BOOK_STATE_ADD: i32 = 1;
pub const RECIPE_BOOK_STATE_REMOVE: i32 = 2;

/// Book settings in `RecipeBookType` declaration order: crafting, furnace,
/// blast furnace, smoker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecipeBookSettings {
    pub crafting_open: bool,
    pub crafting_filtering: bool,
    pub furnace_open: bool,
    pub furnace_filtering: bool,
    pub blast_furnace_open: bool,
    pub blast_furnace_filtering: bool,
    pub smoker_open: bool,
    pub smoker_filtering: bool,
}

#[java_packet(UNLOCK_RECIPES)]
pub struct CUnlockRecipes {
    /// One of [`RECIPE_BOOK_STATE_INIT`], [`RECIPE_BOOK_STATE_ADD`] or
    /// [`RECIPE_BOOK_STATE_REMOVE`].
    pub state: i32,
    pub settings: RecipeBookSettings,
    pub recipes: Vec<Identifier>,
    /// Only sent for INIT.
    pub highlight: Vec<Identifier>,
}

impl CUnlockRecipes {
    /// The INIT packet vanilla sends when a player joins: book settings plus
    /// the known recipes (new players know none).
    #[must_use]
    pub const fn init(settings: RecipeBookSettings, recipes: Vec<Identifier>) -> Self {
        Self {
            state: RECIPE_BOOK_STATE_INIT,
            settings,
            recipes,
            highlight: Vec::new(),
        }
    }
}

impl ClientPacket for CUnlockRecipes {
    fn write_packet_data(
        &self,
        mut write: impl Write,
        _version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_var_int(&VarInt(self.state))?;
        let settings = &self.settings;
        for open in [
            settings.crafting_open,
            settings.furnace_open,
            settings.blast_furnace_open,
            settings.smoker_open,
        ] {
            write.write_bool(open)?;
        }
        for filtering in [
            settings.crafting_filtering,
            settings.furnace_filtering,
            settings.blast_furnace_filtering,
            settings.smoker_filtering,
        ] {
            write.write_bool(filtering)?;
        }

        write.write_var_int(&VarInt(self.recipes.len() as i32))?;
        for recipe in &self.recipes {
            write.write_string(&recipe.to_string())?;
        }
        if self.state == RECIPE_BOOK_STATE_INIT {
            write.write_var_int(&VarInt(self.highlight.len() as i32))?;
            for recipe in &self.highlight {
                write.write_string(&recipe.to_string())?;
            }
        }
        Ok(())
    }
}

impl<'a> ServerPacket<'a> for CUnlockRecipes {
    fn read(bytebuf: &mut &'a [u8], _version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        let state = bytebuf.get_var_int()?.0;
        let mut opens = [false; 4];
        for open in &mut opens {
            *open = bytebuf.get_bool()?;
        }
        let mut filterings = [false; 4];
        for filtering in &mut filterings {
            *filtering = bytebuf.get_bool()?;
        }
        let settings = RecipeBookSettings {
            crafting_open: opens[0],
            furnace_open: opens[1],
            blast_furnace_open: opens[2],
            smoker_open: opens[3],
            crafting_filtering: filterings[0],
            furnace_filtering: filterings[1],
            blast_furnace_filtering: filterings[2],
            smoker_filtering: filterings[3],
        };

        let recipes = read_identifier_list(bytebuf)?;
        let highlight = if state == RECIPE_BOOK_STATE_INIT {
            read_identifier_list(bytebuf)?
        } else {
            Vec::new()
        };
        Ok(Self {
            state,
            settings,
            recipes,
            highlight,
        })
    }
}

fn read_identifier_list(bytebuf: &mut &[u8]) -> Result<Vec<Identifier>, ReadingError> {
    // Mirrors the vanilla list bound of 65536 entries.
    const MAX_RECIPES: i32 = 65536;
    let len = bytebuf.get_var_int()?.0;
    if !(0..=MAX_RECIPES).contains(&len) {
        return Err(ReadingError::Message("recipe list out of bounds".into()));
    }
    let mut recipes = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let id = bytebuf.get_str()?;
        let Some(id) = Identifier::parse(&id).ok() else {
            return Err(ReadingError::Message(format!("invalid recipe id {id}")));
        };
        recipes.push(id);
    }
    Ok(recipes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_util::version::JavaMinecraftVersion;

    #[test]
    fn init_packet_round_trips() {
        let packet = CUnlockRecipes::init(
            RecipeBookSettings {
                crafting_open: true,
                smoker_filtering: true,
                ..RecipeBookSettings::default()
            },
            vec![
                Identifier::parse("minecraft:oak_planks_from_oak_log")
                    .ok()
                    .unwrap(),
            ],
        );
        let mut bytes = Vec::new();
        packet
            .write_packet_data(&mut bytes, &JavaMinecraftVersion::V_1_21)
            .unwrap();

        let mut cursor: &[u8] = &bytes;
        let read = CUnlockRecipes::read(&mut cursor, &JavaMinecraftVersion::V_1_21).unwrap();
        assert_eq!(read.state, RECIPE_BOOK_STATE_INIT);
        assert!(read.settings.crafting_open);
        assert!(read.settings.smoker_filtering);
        assert!(!read.settings.furnace_open);
        assert_eq!(
            read.recipes,
            vec![
                Identifier::parse("minecraft:oak_planks_from_oak_log")
                    .ok()
                    .unwrap()
            ]
        );
        assert!(read.highlight.is_empty());
        assert!(cursor.is_empty());
    }

    #[test]
    fn add_packet_has_no_highlight_list() {
        let packet = CUnlockRecipes {
            state: RECIPE_BOOK_STATE_ADD,
            settings: RecipeBookSettings::default(),
            recipes: Vec::new(),
            highlight: Vec::new(),
        };
        let mut bytes = Vec::new();
        packet
            .write_packet_data(&mut bytes, &JavaMinecraftVersion::V_1_21)
            .unwrap();

        // state varint + 8 setting bools + empty recipe list
        assert_eq!(bytes.len(), 1 + 8 + 1);
    }
}
