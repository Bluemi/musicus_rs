use crate::song::SongID;
use std::path::Path;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Playlist {
	pub name: String,
	pub songs: Vec<SongID>,
	pub cursor_position: usize,
	pub scroll_position: usize,
}

impl Playlist {
	pub fn new(name: String) -> Playlist {
		Playlist {
			name,
			songs: Vec::new(),
			cursor_position: 0,
			scroll_position: 0,
		}
	}

	pub fn from_file(path: &Path) -> Playlist {
		let file = File::open(path).unwrap();
		let reader = BufReader::new(file);
		serde_json::from_reader(reader).unwrap()
	}

	pub fn dump_to_file(&self, path: &Path) {
		let file = OpenOptions::new()
			.write(true)
			.truncate(true)
			.create(true)
			.open(path)
			.unwrap();
		let writer = BufWriter::new(file);
		serde_json::to_writer_pretty(writer, &self).unwrap();
	}

	pub fn set_cursor_position(&mut self, cursor_position: usize, num_rows: usize) {
		self.cursor_position = cursor_position;
		self.normalize_scroll_position(num_rows)
	}

	pub fn normalize_scroll_position(&mut self, num_rows: usize) {
		let scroll_position = self.scroll_position as i32;
		self.scroll_position = scroll_position.clamp(
			self.cursor_position as i32 - num_rows as i32 + 1,
			self.cursor_position as i32
		) as usize;
	}
}
