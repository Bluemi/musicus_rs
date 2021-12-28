use std::collections::HashMap;
use std::path::{PathBuf, Path};
use rodio::{Decoder, Source};
use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;
use crossbeam::{Sender, Receiver, unbounded};
use std::thread;
use crate::audio_backend::arc_samples_buffer::{Sound, ArcSamplesBuffer};
use std::time::Duration;
use crate::audio_backend::chunk::{CHUNK_SIZE, SamplesChunk};
use crate::song::{Song, SongID};

pub type SongBuffer = HashMap<SongID, Sound>;

const BUFFER_SIZE: usize = 5; // buffer at most 5 songs. If 6 songs are loaded -> garbage collect

pub struct AudioBuffer {
    songs: SongBuffer,
    load_sender: Sender<Song>,
}

#[derive(Debug)]
pub enum OpenError {
    FileNotFound,
    NotDecodable,
}

enum LoadInfo {
    Chunk(SamplesChunk),
    Duration(SongID, Duration),
    Err(OpenError),
}

/**
 * Loads chunks of songs, initiated through the load_receiver channel
 */
fn load_chunks(load_receiver: Receiver<Song>, chunk_sender: Sender<LoadInfo>) {
    'l: for song in load_receiver.iter() {
        if let Ok(file) = File::open(&song.get_path()) {
            if let Ok(decoder) = Decoder::new(BufReader::new(file)) {
                let channels = decoder.channels();
                let sample_rate = decoder.sample_rate();
                let duration = decoder.total_duration();

                if let Some(duration) = duration {
                    let _ = chunk_sender.send(LoadInfo::Duration(song.get_id(), duration));
                }

                let data = Box::new([0.0f32; CHUNK_SIZE]);
                let mut index = 0;
                let mut next_start_position = 0;
                for sample in decoder.convert_samples() {
                    let chunk_index = index % CHUNK_SIZE;
                    data[chunk_index] = sample;

                    // send chunk
                    if chunk_index == CHUNK_SIZE-1 {
                        let chunk = SamplesChunk {
                            channels,
                            sample_rate,
                            start_position: next_start_position,
                            length: CHUNK_SIZE,
                            data: data.clone(),
                            song: song.clone(),
                        };
                        next_start_position = index + 1;
                        if chunk_sender.send(LoadInfo::Chunk(chunk)).is_err() {
                            break 'l
                        }
                    }
                    index += 1;
                }
                let chunk_index = index % CHUNK_SIZE;
                if chunk_index != 0 {
                    let chunk = SamplesChunk {
                        channels,
                        sample_rate,
                        start_position: next_start_position,
                        length: index - next_start_position,
                        data: data.clone(),
                        song: song.clone(),
                    };
                    if chunk_sender.send(LoadInfo::Chunk(chunk)).is_err() {
                        break 'l
                    }
                }
            } else {
                let _ = chunk_sender.send(LoadInfo::Err(OpenError::NotDecodable));
            }
        } else {
            let _ = chunk_sender.send(LoadInfo::Err(OpenError::FileNotFound));
        }
    }
}

// TODO: If background thread loads song it is possible to load it two times.
impl AudioBuffer {
    /**
     * Creates a new AudioBuffer
     */
    pub fn new(chunk_sender: Sender<LoadInfo>) -> AudioBuffer {
        let (load_sender, load_receiver): (Sender<Song>, Receiver<Song>) = unbounded();

        // thread that loads buffers in background
        thread::spawn(move || {
            load_chunks(load_receiver, chunk_sender);
        });

        AudioBuffer {
            songs: HashMap::new(),
            load_sender,
        }
    }

    /**
     * Initiates the loading of the given path. Does return immediately.
     */
    #[allow(unused)]
    pub fn load(&mut self, song: Song) {
        self.load_sender.send(song).unwrap();
    }

    /**
     * Loads the given path and makes it available in the contained hashmap. Blocks until the song is
     * loaded.
     */
    pub fn load_blocking(songs: &SongBuffer, path: PathBuf, counter: usize) -> Result<ArcSamplesBuffer, OpenError> {
        if let Ok(file) = File::open(&path) {
            if let Ok(source) = Decoder::new(BufReader::new(file)) {
                let channels = source.channels();
                let sample_rate = source.sample_rate();
                let duration = source.total_duration();
                let data: Vec<f32> = source.convert_samples().collect();

                let duration = duration.unwrap_or_else(|| Duration::from_secs_f64(data.len() as f64 / sample_rate as f64 / channels as f64));
                let sound = Sound {
                    channels,
                    sample_rate,
                    duration,
                    data,
                    counter,
                };
                let arc = Arc::new(sound);
                songs.insert(path, arc.clone());
                Ok(ArcSamplesBuffer::new(arc))
            } else {
                Err(OpenError::NotDecodable)
            }
        } else {
            Err(OpenError::FileNotFound)
        }
    }

    /**
     * Returns a sample buffer. If the buffer was not loaded it is loaded after this function.
     */
    pub fn get(&mut self, path: &Path) -> Result<ArcSamplesBuffer, OpenError> {
        if !self.songs.contains_key(path) {
            let res = Self::load_blocking(&self.songs, path.to_path_buf(), self.song_counter);
            if res.is_ok() {
                self.song_counter += 1;
            }
            return res
        }
        let sound = self.songs.get(path).unwrap();
        Ok(ArcSamplesBuffer::new(sound.value().clone()))
    }

    pub fn check_garbage_collect(&mut self) -> Option<PathBuf> {
        if self.songs.len() > BUFFER_SIZE {
            if let Some(key) = self.songs.iter().min_by(|a, b| a.counter.cmp(&b.counter)).map(|e| e.key().clone()) {
                return self.songs.remove(&key).map(|i| i.0);
            }
        }
        None
    }
}