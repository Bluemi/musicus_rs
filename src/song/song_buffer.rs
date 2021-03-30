use crate::song::{Song, SongID, title_from_path};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs::{OpenOptions, File};
use crate::config::get_song_buffer_path;
use std::io::{BufWriter, BufReader};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
pub struct SongBuffer {
	songs: Vec<Song>,
	next_id: SongID,
}

impl SongBuffer {
	pub fn new() -> SongBuffer {
		SongBuffer {
			songs: Vec::new(),
			next_id: 0,
		}
	}

	pub fn import(&mut self, path: &Path, title: Option<&str>) -> SongID {
		if let Some(song) = self.get_mut_by_path(path) {
			return song.get_id();
		}
		self.import_new(path, title)
	}

	fn import_new(&mut self, path: &Path, title: Option<&str>) -> SongID {
		let title = title.map(|t| t.to_string()).unwrap_or_else(|| title_from_path(path));
		let id = self.next_id;
		let song = Song {
			id,
			title,
			path: path.to_path_buf(),
			total_duration: None,
		};
		self.next_id += 1;
		self.songs.push(song);
		id
	}

	pub fn get(&self, id: SongID) -> Option<&Song> {
		self.songs.iter().find(|s| s.get_id() == id)
	}

	#[allow(unused)]
	pub fn get_mut(&mut self, id: SongID) -> Option<&mut Song> {
		self.songs.iter_mut().find(|s| s.get_id() == id)
	}

	#[allow(unused)]
	pub fn get_by_path(&self,path: &Path) -> Option<&Song> {
		self.songs.iter().find(|s| s.get_path() == path)
	}

	pub fn get_mut_by_path(&mut self, path: &Path) -> Option<&mut Song> {
		self.songs.iter_mut().find(|s| s.get_path() == path)
	}

	pub fn update_total_duration(&mut self, song_id: SongID, duration: Duration) {
		if let Some(song) = self.get_mut(song_id) {
			song.update_total_duration(duration);
		}
	}

	pub fn dump(&self) {
		let file = OpenOptions::new()
			.write(true)
			.truncate(true)
			.create(true)
			.open(get_song_buffer_path())
			.unwrap();
		let writer = BufWriter::new(file);
		serde_json::to_writer_pretty(writer, &self).unwrap();
	}

	pub fn load() -> Result<SongBuffer, ()> {
		let song_buffer_path = get_song_buffer_path();
		if song_buffer_path.is_file() {
			if let Ok(file) = File::open(song_buffer_path) {
				let reader = BufReader::new(file);
				if let Ok(song_buffer) = serde_json::from_reader(reader) {
					Ok(song_buffer)
				} else {
					Err(())
				}
			} else {
				Err(())
			}
		} else {
			Ok(SongBuffer::new())
		}
	}
}