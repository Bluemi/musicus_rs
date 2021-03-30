pub mod song_buffer;

use std::ffi::OsString;
use std::path::{PathBuf, Path};
use crate::file_manager::file_utils::{get_dir_entries, DirectoryEntry, get_common_ends};
use std::fmt::{Debug, Formatter};
use serde::{Serialize, Deserialize};
use crate::playlists::normalize_title;
use crate::song::song_buffer::SongBuffer;
use std::time::Duration;

pub type SongID = u32;

#[derive(Serialize, Deserialize, Clone)]
pub struct Song {
	id: SongID,
	title: String,
	path: PathBuf,
	total_duration: Option<Duration>,
}

impl Song {
	pub fn get_id(&self) -> SongID {
		self.id
	}

	pub fn get_title(&self) -> &str {
		&self.title
	}

	pub fn get_path(&self) -> &Path {
		&self.path
	}

	pub fn get_total_duration(&self) -> Option<Duration> {
		self.total_duration
	}

	pub fn update_total_duration(&mut self, duration: Duration) {
		self.total_duration = Some(duration);
	}

	pub fn songs_from_path(path: &Path, song_buffer: &mut SongBuffer) -> Vec<SongID> {
		let dir_entries = get_dir_entries(path);
		let sound_files: Vec<&DirectoryEntry> = dir_entries.iter().filter(|de| de.is_song_file()).collect();
		let sub_directories: Vec<&DirectoryEntry> = dir_entries.iter().filter(|de| !de.is_file).collect();

		let mut songs = Song::songs_from_sound_files(sound_files, song_buffer);

		for sub_directory in sub_directories {
			songs.extend(Song::songs_from_path(&sub_directory.path, song_buffer));
		}

		songs
	}

	pub fn songs_from_sound_files(sound_files: Vec<&DirectoryEntry>, song_buffer: &mut SongBuffer) -> Vec<SongID> {
		let (mut start, mut end) = ("", "");
		// matching same name parts only makes sense for more than one song
		if sound_files.len() > 1 {
			(start, end) = get_common_ends(sound_files.iter().map(|de| &*de.filename)).unwrap();
		}

		let mut songs = Vec::new();

		for (index, sound_file) in sound_files.iter().enumerate() {
			let title = &sound_file.filename[start.len()..sound_file.filename.len()-end.len()];
			let title = normalize_title(title, index+1);

			let id = song_buffer.import(&sound_file.path, Some(&title));

			songs.push(id);
		};
		songs
	}
}

pub fn title_from_path(path: &Path) -> String {
	path.file_name().unwrap_or(&OsString::from("<no filename>")).to_string_lossy().into_owned()
}

impl Debug for Song {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.title)
	}
}

