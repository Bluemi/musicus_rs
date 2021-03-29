use crate::song::{Song, SongID, title_from_path};
use serde::{Serialize, Deserialize};
use std::path::Path;

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
		};
		self.next_id += 1;
		self.songs.push(song);
		id
	}

	pub fn get(&self, id: SongID) -> Option<&Song> {
		self.songs.iter().find(|s| s.get_id() == id)
	}

	pub fn get_mut(&mut self, id: SongID) -> Option<&mut Song> {
		self.songs.iter_mut().find(|s| s.get_id() == id)
	}

	pub fn get_by_path(&self,path: &Path) -> Option<&Song> {
		self.songs.iter().find(|s| s.get_path() == path)
	}

	pub fn get_mut_by_path(&mut self,path: &Path) -> Option<&mut Song> {
		self.songs.iter_mut().find(|s| s.get_path() == path)
	}
}