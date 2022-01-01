use std::time::Duration;
use crossbeam::{Receiver, Sender};
use rodio::Source;
use crate::audio_backend::{AudioBackendCommand, AudioUpdate, PlayingUpdate};
use crate::audio_backend::chunk::SamplesChunk;

const SOFT_FADEOUT_DECAY: f32 = 0.01;


pub struct ReceiverSource {
    chunk_receiver: Receiver<SamplesChunk>, // Receiver of the chunks to play
    update_sender: Sender<AudioBackendCommand>, // Send AudioUpdates to Backend
    current_chunk: Option<SamplesChunk>,
    samples_counter: usize, // points to the current position in current_chunk
    last_value: f32,
}

impl ReceiverSource {
    pub fn new(chunk_receiver: Receiver<SamplesChunk>, update_sender: Sender<AudioBackendCommand>) -> ReceiverSource {
        ReceiverSource {
            chunk_receiver,
            update_sender,
            current_chunk: None,
            samples_counter: 0,
            last_value: 0.0,
        }
    }

    #[inline]
    fn load_next_chunk(&mut self) {
        match self.chunk_receiver.try_recv() {
            Ok(chunk) => {
                // check for new song
                if self.current_chunk.as_ref().map(|c| c.song_id != chunk.song_id).unwrap_or(true) {
                    let _ = self.update_sender.send(
                        AudioBackendCommand::Update(AudioUpdate::SongStarts(
                            chunk.song_id
                        ))
                    );
                } else {
                    // inform backend for every chunk, only if not new song
                    let _ = self.update_sender.send(
                        AudioBackendCommand::Update(AudioUpdate::Playing(
                            PlayingUpdate {
                                song_id: chunk.song_id,
                                samples_played: chunk.start_position,
                            }
                        ))
                    );
                }

                self.current_chunk = Some(chunk);
                self.samples_counter = 0;
            }
            Err(_) => {
                self.current_chunk = None;
                self.samples_counter = 0;
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
                if self.samples_counter == current_chunk.data.len() {
                    self.load_next_chunk();
                }
            }
            None => self.load_next_chunk(),
        }

        // use value from current chunk or do soft fadeout
        let value = match &self.current_chunk {
            Some(chunk) => {
                let val = chunk.data[self.samples_counter];
                self.samples_counter += 1;
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
