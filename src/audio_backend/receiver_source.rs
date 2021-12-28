use std::time::Duration;
use crossbeam::{Receiver, Sender, TryRecvError};
use rodio::Source;
use crate::audio_backend::{AudioBackendCommand, AudioUpdate, PlayingUpdate};
use crate::audio_backend::chunk::SamplesChunk;
use crate::song::Song;

const SOFT_FADEOUT_DECAY: f32 = 0.01;
const PERIODIC_ACCESS_UPDATE_PERIOD: Duration = Duration::from_millis(100);


pub struct ReceiverSource {
    samples_receiver: Receiver<SamplesChunk>, // Receiver of the chunks to play
    update_sender: Sender<AudioBackendCommand>, // Send AudioUpdates to Backend
    current_chunk: Option<SamplesChunk>,
    counter: usize, // points to the current position in current_chunk
    last_value: f32,
    periodic_access: PeriodicAccess,
}

struct PeriodicAccess {
    update_frequency: u32,
    samples_until_update: u32,
    duration_played: Duration,
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
                // check for new song
                if self.current_chunk.as_ref().map(|c| c.song.get_id() != chunk.song.get_id()).unwrap_or(true) {
                    // TODO send SongStarts update
                    /*
                    self.update_sender.send(
                        AudioBackendCommand::Update(AudioUpdate::SongStarts(
                            chunk.song.clone(), skip.unwrap_or_else(|| Duration::new(0, 0))
                        ))
                    )
					 */
                }

                //
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
            Err(TryRecvError::Empty) => {
                self.current_chunk = None;
                self.counter = 0;
                self.periodic_access = PeriodicAccess {
                    update_frequency: 0,
                    samples_until_update: 0,
                    duration_played: Duration::new(0, 0),
                }
            }
            _ => todo!()
        }
    }

    fn tick_periodic_access(&mut self) {
        if let Some(chunk) = &self.current_chunk {
            if self.periodic_access.tick() {
                self.update_sender.send(
                    AudioBackendCommand::Update(AudioUpdate::Playing(
                        PlayingUpdate {
                            song: chunk.song.clone(),
                            duration_played: self.periodic_access.duration_played,
                        }
                    )
                    ));
            }
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
                let val = chunk.data[self.counter];
                self.counter += 1;
                val
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

        self.tick_periodic_access();

        self.last_value = value;
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

impl PeriodicAccess {
    fn tick(&mut self) -> bool {
        self.samples_until_update -= 1;
        let update = self.samples_until_update == 0;
        if update {
            self.duration_played += PERIODIC_ACCESS_UPDATE_PERIOD;
            self.samples_until_update = self.update_frequency;
        }
        update
    }
}