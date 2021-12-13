use std::path::{PathBuf, Path};
use rodio::{Decoder, Source};
use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;
use crossbeam::{Sender, Receiver, unbounded};
use std::thread;
use dashmap::DashMap;
use crate::audio_backend::arc_samples_buffer::{Sound, ArcSamplesBuffer};
use std::time::Duration;

pub type ArcSongBuffer = Arc<DashMap<PathBuf, Arc<Sound>>>;

const BUFFER_SIZE: usize = 5; // buffer at most 5 songs. If 6 songs are loaded -> garbage collect

pub struct AudioBuffer {
    songs: ArcSongBuffer,
    load_sender: Sender<(PathBuf, usize)>,
    song_counter: usize,
}

#[derive(Debug)]
pub enum OpenError {
    FileNotFound,
    NotDecodable,
}

// TODO: If background thread loads song it is possible to load it two times.
impl AudioBuffer {
    /**
     * Creates a new AudioBuffer
     */
    pub fn new() -> AudioBuffer {
        let (load_sender, load_receiver): (Sender<(PathBuf, usize)>, Receiver<(PathBuf, usize)>) = unbounded();

        let songs = Arc::new(DashMap::new());

        let load_songs = songs.clone();

        // thread that loads buffers in background
        thread::spawn(move || {
            for (path, counter) in load_receiver.iter() {
                if !load_songs.contains_key(&path) {
                    Self::load_blocking(&load_songs, path, counter).ok(); // TODO: send failure to main thread
                }
            }
        });

        AudioBuffer {
            songs,
            load_sender,
            song_counter: 0,
        }
    }

    /**
     * Initiates the loading of the given path. Does return immediately.
     */
    #[allow(unused)]
    pub fn load(&mut self, path: PathBuf) {
        self.load_sender.send((path, self.song_counter)).unwrap();
        self.song_counter += 1;
    }

    /**
     * Loads the given path and makes it available in the contained hashmap. Blocks until the song is
     * loaded.
     */
    pub fn load_blocking(songs: &ArcSongBuffer, path: PathBuf, counter: usize) -> Result<ArcSamplesBuffer, OpenError> {
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