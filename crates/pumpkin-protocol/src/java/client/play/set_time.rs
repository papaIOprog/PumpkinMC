use pumpkin_data::packet::clientbound::play::SET_TIME;
use pumpkin_macros::java_packet;
use pumpkin_util::version::JavaMinecraftVersion;

use crate::{
    ClientPacket,
    ser::{NetworkWriteExt, WritingError},
};

#[java_packet(SET_TIME)]
pub struct CUpdateTime {
    pub game_time: i64,
    /// (`clock_registry_id`, `total_ticks`, `partial_tick`, rate)
    pub clock_updates: Vec<(i32, i64, f32, f32)>,
}

impl CUpdateTime {
    #[must_use]
    pub fn new(game_time: i64, day_time: i64, increasing: bool) -> Self {
        let overworld_id = 0;
        let partial_tick = 0.0f32;
        let rate = if increasing { 1.0f32 } else { 0.0f32 }; // Normal speed
        Self {
            game_time,
            clock_updates: vec![(overworld_id, day_time, partial_tick, rate)],
        }
    }

    #[must_use]
    pub fn new_clock(
        game_time: i64,
        clock_id: i32,
        total_ticks: i64,
        partial_tick: f32,
        rate: f32,
    ) -> Self {
        Self {
            game_time,
            clock_updates: vec![(clock_id, total_ticks, partial_tick, rate)],
        }
    }
}

impl ClientPacket for CUpdateTime {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_i64_be(self.game_time)?;

        let _ = version;
        let (day_time, rate) = self
            .clock_updates
            .first()
            .map_or((0, 1.0), |&(_, total_ticks, _, rate)| (total_ticks, rate));

        let is_paused = rate == 0.0;
        let day_time = if is_paused {
            match day_time.cmp(&0) {
                std::cmp::Ordering::Greater => -day_time,
                std::cmp::Ordering::Equal => -1,
                std::cmp::Ordering::Less => day_time,
            }
        } else {
            day_time.abs()
        };

        write.write_i64_be(day_time)?;

        Ok(())
    }
}
