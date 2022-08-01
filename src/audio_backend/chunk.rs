use std::sync::Arc;
use std::time::Duration;
use crate::song::SongID;

pub const CHUNK_SIZE: usize = 1024;

#[derive(Clone, Debug)]
pub struct SamplesChunk {
    pub channels: u16,
    pub sample_rate: u32,
    /// The position of this chunk in the song as number of samples. A sample is one f32 value.
    pub start_position: usize,
    /// The number of samples in this chunk. A sample is one f32 value.
    pub length: usize,
    pub data: Arc<[f32; CHUNK_SIZE]>,
    pub song_id: SongID,
    pub last_chunk: bool,
}

impl SamplesChunk {
    #[allow(unused)]
    pub fn get_start_time(&self) -> Duration {
        position_to_duration(self.start_position, self.sample_rate, self.channels)
    }

    #[allow(unused)]
    pub fn get_end_time(&self) -> Duration {
        position_to_duration(self.start_position + self.length, self.sample_rate, self.channels)
    }
}

pub fn position_to_duration(position: usize, sample_rate: u32, channels: u16) -> Duration {
    Duration::from_micros(position as u64 * 1_000_000u64 / sample_rate as u64 / channels as u64)
}

pub fn duration_to_position(duration: &Duration, sample_rate: u32, channels: u16) -> usize {
    (duration.as_micros() * sample_rate as u128 * channels as u128 / 1_000_000u128) as usize
}