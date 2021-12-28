use std::time::Duration;
use crate::song::Song;

pub const CHUNK_SIZE: usize = 1024;

pub struct SamplesChunk {
    pub channels: u16,
    pub sample_rate: u32,
    /**
     * The position of this chunk in the song as number of samples. A sample is one f32 value.
     */
    pub start_position: usize,
    /**
     * The number of samples in this chunk. A sample is one f32 value.
     */
    pub length: usize,
    pub data: Box<[f32; CHUNK_SIZE]>,
    pub song: Song,
}

impl SamplesChunk {
    pub fn get_start_time(&self) -> Duration {
        let nanos = self.start_position as u64 * 1_000_000u64 / self.sample_rate as u64 / self.channels as u64;
        Duration::from_nanos(nanos)
    }

    pub fn get_end_time(&self) -> Duration {
        let nanos = (self.start_position + self.length) as u64 * 1_000_000u64 / self.sample_rate as u64 / self.channels as u64;
        Duration::from_nanos(nanos)
    }
}