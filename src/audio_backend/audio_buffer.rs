use std::path::{PathBuf, Path};
use rodio::{buffer, Decoder, Source};
use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;
use crossbeam::{Sender, Receiver, unbounded};
use std::thread;
use dashmap::DashMap;
use rodio::buffer::SamplesBuffer;

pub type SongBuffer = Arc<buffer::SamplesBuffer<f32>>;
pub type ArcSongBuffer = Arc<DashMap<PathBuf, Arc<Sound>>>;

pub struct Sound {
	channels: u16,
	sample_rate: u32,
	data: Vec<f32>,
}

pub struct AudioBuffer {
	songs: ArcSongBuffer,
	load_sender: Sender<PathBuf>,
}

impl AudioBuffer {
	pub fn new() -> AudioBuffer {
		let (load_sender, load_receiver): (Sender<PathBuf>, Receiver<PathBuf>) = unbounded();

		let songs = Arc::new(DashMap::new());

		let load_songs = songs.clone();

		// thread that loads buffers in background
		thread::spawn(move || {
			for path in load_receiver.iter() {
				if !load_songs.contains_key(&path) {
					Self::load_blocking(&load_songs, path);
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
	pub fn load(&self, path: PathBuf) {
		self.load_sender.send(path).unwrap();
	}

	/**
	 * Loads the given path and makes it available in the contained hashmap. Blocks until the song is
	 * loaded.
	 */
	pub fn load_blocking(songs: &ArcSongBuffer, path: PathBuf) -> SamplesBuffer<f32> {
		// TODO: handle wrong files
		let file = File::open(&path).unwrap();
		let source = Decoder::new(BufReader::new(file)).unwrap();
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
		return SamplesBuffer::new(channels, sample_rate, arc.data.clone());
	}

	pub fn get(&self, path: &Path) -> SamplesBuffer<f32> {
		if !self.songs.contains_key(path) {
			return Self::load_blocking(&self.songs, path.to_path_buf());
		}
		let sound = self.songs.get(path).unwrap();
		return SamplesBuffer::new(sound.channels, sound.sample_rate, sound.data.clone());
	}
}