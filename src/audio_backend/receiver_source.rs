use std::time::Duration;
use crossbeam::{Receiver, Sender, TryRecvError};
use rodio::Source;
use crate::audio_backend::AudioBackendCommand;

const CHUNK_SIZE: usize = 1024;
const SOFT_FADEOUT_DECAY: f32 = 0.01;
const PERIODIC_ACCESS_UPDATE_PERIOD: Duration = Duration::from_millis(100);

pub struct SamplesChunk {
    pub channels: u16,
    pub sample_rate: u32,
    pub duration: Duration,
    pub data: Box<[f32; CHUNK_SIZE]>,
    pub first_chunk: bool,
    pub last_chunk: bool,
}

struct PeriodicAccess {
    update_frequency: u32,
    samples_until_update: u32,
    duration_played: Duration,
}

pub struct ReceiverSource {
    samples_receiver: Receiver<SamplesChunk>, // Receiver of the chunks to play
    update_sender: Sender<AudioBackendCommand>, // Send AudioUpdates to Backend
    current_chunk: Option<SamplesChunk>,
    counter: usize, // points to the current position in current_chunk
    last_value: f32,
    periodic_access: PeriodicAccess,
}

impl ReceiverSource {
    pub fn new(samples_receiver: Receiver<SamplesChunk>, update_sender: Sender<AudioBackendCommand>) -> ReceiverSource {
        ReceiverSource {
            samples_receiver,
            update_sender,
            current_chunk: None,
            counter: 0,
            last_value: 0.0,
            periodic_access: PeriodicAccess {
                update_frequency: 0,
                samples_until_update: 1,
                duration_played: Duration::new(0, 0),
            }
        }
    }

    fn load_next_chunk(&mut self) {
        match self.samples_receiver.try_recv() {
            Ok(chunk) => {
                let update_frequency = (PERIODIC_ACCESS_UPDATE_PERIOD.as_secs_f64() * chunk.sample_rate as f64 * chunk.channels as f64) as u32;

                self.current_chunk = Some(chunk);
                self.counter = 0;
                // periodic access
                self.periodic_access = PeriodicAccess {
                    update_frequency,
                    samples_until_update: self.periodic_access.samples_until_update,
                    duration_played: Duration::new(0, 0),
                }
            }
            Err(TryRecvError::Empty) => {}
            _ => todo!()
        }
    }
}

impl Iterator for ReceiverSource {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // try to load next chunk, if no current chunk or current chunk completely played
        match &self.current_chunk {
            Some(current_chunk) => {
                // if the current chunk was completely played
                if self.counter == current_chunk.data.len() {
                    self.load_next_chunk();
                }
            }
            None => self.load_next_chunk(),
        }
        // use value from current chunk or do soft fadeout
        let value = match &self.current_chunk {
            Some(chunk) => {
                chunk.data[self.counter]
            }
            None => {
                // soft fade out
                if self.last_value.abs() < SOFT_FADEOUT_DECAY {
                    0.0
                } else {
                    self.last_value - self.last_value.signum() * SOFT_FADEOUT_DECAY
                }
            }
        };
        self.last_value = value;

        self.counter += 1;
        Some(value)
    }
}

impl Source for ReceiverSource {
    fn current_frame_len(&self) -> Option<usize> {
        self.current_chunk.as_ref().map(|chunk| chunk.data.len())
    }

    fn channels(&self) -> u16 {
        self.current_chunk.as_ref().map(|chunk| chunk.channels).unwrap_or(2)
    }

    fn sample_rate(&self) -> u32 {
        self.current_chunk.as_ref().map(|chunk| chunk.sample_rate).unwrap_or(44100)
    }

    fn total_duration(&self) -> Option<Duration> {
        None // this source plays for ever
    }
}