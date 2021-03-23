use std::path::{PathBuf, Path};
use crate::file_manager::file_utils::{get_dir_entries, DirectoryEntry, get_common_ends};
use std::fmt::{Debug, Formatter};
use serde::{Serialize, Deserialize};
use crate::playlists::normalize_title;

#[derive(Serialize, Deserialize, Clone)]
pub struct Song {
	pub title: String,
	pub path: PathBuf,
}

impl Song {
	pub fn songs_from_path(path: &Path) -> Vec<Song> {
		let dir_entries = get_dir_entries(path);
		let sound_files: Vec<&DirectoryEntry> = dir_entries.iter().filter(|de| de.is_song_file()).collect();
		let sub_directories: Vec<&DirectoryEntry> = dir_entries.iter().filter(|de| !de.is_file).collect();

		let mut songs = Song::songs_from_sound_files(sound_files);

		for sub_directory in sub_directories {
			songs.extend(Song::songs_from_path(&sub_directory.path));
		}

		songs
	}

	pub fn songs_from_sound_files(sound_files: Vec<&DirectoryEntry>) -> Vec<Song> {
		let (mut start, mut end) = ("", "");
		// matching same name parts only makes sense for more than one song
		if sound_files.len() > 1 {
			(start, end) = get_common_ends(sound_files.iter().map(|de| &*de.filename)).unwrap();
		}

		let mut songs = Vec::new();

		for (index, sound_file) in sound_files.iter().enumerate() {
			let title = &sound_file.filename[start.len()..sound_file.filename.len()-end.len()];
			let title = normalize_title(title, index+1);

			songs.push(
				Song {
					title,
					path: sound_file.path.clone(),
				}
			);
		};
		songs
	}
}

impl Debug for Song {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.title)
	}
}

