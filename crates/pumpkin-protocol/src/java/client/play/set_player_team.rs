use std::io::Write;

use pumpkin_data::packet::clientbound::play::SET_PLAYER_TEAM;
use pumpkin_macros::java_packet;
use pumpkin_util::{text::TextComponent, translation::Locale, version::JavaMinecraftVersion};

use crate::{
    ClientPacket,
    codec::var_int::VarInt,
    ser::{NetworkWriteExt, WritingError},
};

#[repr(i8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamMethod {
    Create = 0,
    Remove = 1,
    Update = 2,
    AddPlayers = 3,
    RemovePlayers = 4,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamParameters<'a> {
    pub display_name: &'a TextComponent,
    pub options: i8,
    pub nametag_visibility: &'a str,
    pub collision_rule: &'a str,
    pub color: i32,
    pub player_prefix: &'a TextComponent,
    pub player_suffix: &'a TextComponent,
}

#[java_packet(SET_PLAYER_TEAM)]
pub struct CSetPlayerTeam<'a> {
    pub team_name: String,
    pub method: TeamMethod,
    pub parameters: Option<TeamParameters<'a>>,
    pub players: Box<[String]>,
}

impl ClientPacket for CSetPlayerTeam<'_> {
    fn write_packet_data(
        &self,
        mut write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_18 {
            write.write_string(&self.team_name)?;
        } else {
            write.write_string_bounded(&self.team_name, 16)?;
        }

        write.write_i8(self.method as i8)?;

        if self.method == TeamMethod::Create || self.method == TeamMethod::Update {
            let Some(params) = &self.parameters else {
                return Err(WritingError::Message(
                    "Parameters missing for Create/Update".into(),
                ));
            };

            if *version >= JavaMinecraftVersion::V_1_13 {
                write.write_component(params.display_name, version)?;
                write.write_i8(params.options)?;
                write.write_string_bounded(params.nametag_visibility, 40)?;
                write.write_string_bounded(params.collision_rule, 40)?;
                if *version >= JavaMinecraftVersion::V_1_17 {
                    let color_id = if params.color < 0 { 21 } else { params.color };
                    write.write_var_int(&VarInt(color_id))?;
                } else {
                    write.write_i8(params.color as i8)?;
                }
                write.write_component(params.player_prefix, version)?;
                write.write_component(params.player_suffix, version)?;
            } else {
                let display_name_legacy = params
                    .display_name
                    .to_legacy_string_for_version(version, Locale::EnUs);
                let player_prefix_legacy = params
                    .player_prefix
                    .to_legacy_string_for_version(version, Locale::EnUs);
                let player_suffix_legacy = params
                    .player_suffix
                    .to_legacy_string_for_version(version, Locale::EnUs);

                write.write_string_bounded(&display_name_legacy, 32)?;
                write.write_string_bounded(&player_prefix_legacy, 32)?;
                write.write_string_bounded(&player_suffix_legacy, 32)?;
                write.write_i8(params.options)?;

                if *version <= JavaMinecraftVersion::V_1_7_6 {
                    // 1.7.x has no nametag visibility, collision rule, or color in team info
                } else {
                    write.write_string_bounded(params.nametag_visibility, 32)?;
                    if *version >= JavaMinecraftVersion::V_1_9 {
                        write.write_string_bounded(params.collision_rule, 32)?;
                    }
                    write.write_i8(params.color as i8)?;
                }
            }
        }

        if self.method == TeamMethod::Create
            || self.method == TeamMethod::AddPlayers
            || self.method == TeamMethod::RemovePlayers
        {
            if *version <= JavaMinecraftVersion::V_1_7_6 {
                write.write_i16_be(self.players.len() as i16)?;
            } else {
                write.write_var_int(&VarInt(self.players.len() as i32))?;
            }
            for player in &self.players {
                write.write_string_bounded(player, 40)?;
            }
        }

        Ok(())
    }
}
