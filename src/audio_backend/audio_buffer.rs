use std::path::{PathBuf, Path};
use rodio::{Decoder, Source};
use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;
use crossbeam::{Sender, Receiver, unbounded};
use std::thread;
use dashmap::DashMap;
use rodio::buffer::SamplesBuffer;

pub type ArcSongBuffer = Arc<DashMap<PathBuf, Arc<Sound>>>;

pub struct AudioBuffer {
	songs: ArcSongBuffer,
	load_sender: Sender<PathBuf>,
}

pub struct Sound {
	channels: u16,
	sample_rate: u32,
	data: Vec<f32>,
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
		let (load_sender, load_receiver): (Sender<PathBuf>, Receiver<PathBuf>) = unbounded();

		let songs = Arc::new(DashMap::new());

		let load_songs = songs.clone();

		// thread that loads buffers in background
		thread::spawn(move || {
			for path in load_receiver.iter() {
				if !load_songs.contains_key(&path) {
					Self::load_blocking(&load_songs, path).ok(); // TODO: send failure to main thread
				}
			}
		});

		AudioBuffer {
			songs,
			load_sender,
		}
	}

	/**
	 * Initiates the loading of the given path. Does return immediately.
	 */
	#[allow(unused)]
	pub fn load(&self, path: PathBuf) {
		self.load_sender.send(path).unwrap();
	}

	/**
	 * Loads the given path and makes it available in the contained hashmap. Blocks until the song is
	 * loaded.
	 */
	pub fn load_blocking(songs: &ArcSongBuffer, path: PathBuf) -> Result<SamplesBuffer<f32>, OpenError> {
		if let Ok(file) = File::open(&path) {
			if let Ok(source) = Decoder::new(BufReader::new(file)) {
				let channels = source.channels();
				let sample_rate = source.sample_rate();
				let data: Vec<f32> = source.convert_samples().collect();
				let sound = Sound {
					channels,
					sample_rate,
					data,
				};
				let new_path = path.clone();
				let arc = Arc::new(sound);
				songs.insert(new_path, arc.clone());
				Ok(SamplesBuffer::new(channels, sample_rate, arc.data.clone()))
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
	pub fn get(&self, path: &Path) -> Result<SamplesBuffer<f32>, OpenError> {
		if !self.songs.contains_key(path) {
			return Self::load_blocking(&self.songs, path.to_path_buf());
		}
		let sound = self.songs.get(path).unwrap();
		Ok(SamplesBuffer::new(sound.channels, sound.sample_rate, sound.data.clone()))
	}
}