use pumpkin_data::packet::clientbound::login::GAME_PROFILE;
use pumpkin_util::version::JavaMinecraftVersion;

use crate::{ClientPacket, Property, packet::MultiVersionJavaPacket, ser::NetworkWriteExt};

/// Sent by the server to signal a successful login and transition to the configuration phase
///
/// This packet provides the client with its official UUID and username as
/// recognized by the server, along with any associated skin or cape properties.
pub struct CLoginSuccess<'a> {
    /// The unique identifier assigned to the player.
    pub uuid: &'a uuid::Uuid,
    /// The player's verified username.
    pub username: &'a str,
    /// A list of properties for the player's profile, such as skin data and signatures.
    /// This is typically retrieved from the Mojang authentication servers.
    pub properties: &'a [Property],
    /// Whether strict error handling should be enabled.
    pub strict_error_handling: bool,
}

impl<'a> CLoginSuccess<'a> {
    #[must_use]
    pub const fn new(
        uuid: &'a uuid::Uuid,
        username: &'a str,
        properties: &'a [Property],
        strict_error_handling: bool,
    ) -> Self {
        Self {
            uuid,
            username,
            properties,
            strict_error_handling,
        }
    }
}

impl MultiVersionJavaPacket for CLoginSuccess<'_> {
    fn to_id(version: JavaMinecraftVersion) -> i32 {
        GAME_PROFILE.to_id(version)
    }
}

impl ClientPacket for CLoginSuccess<'_> {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        if version < &JavaMinecraftVersion::V_1_16 {
            write.write_string(&self.uuid.to_string())?;
        } else {
            write.write_uuid(self.uuid)?;
        }
        write.write_string(self.username)?;
        if version >= &JavaMinecraftVersion::V_1_19 {
            write.write_list(self.properties, |write, property| property.write(write))?;
        }
        if version >= &JavaMinecraftVersion::V_1_20_5 {
            write.write_bool(self.strict_error_handling)?;
        }
        Ok(())
    }
}
