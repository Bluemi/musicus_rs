use std::path::PathBuf;
use crate::file_utils::{create_dir, get_dir_entries};
use crate::playlists::Playlist;

pub fn get_config_directory() -> PathBuf {
	dirs::config_dir().unwrap().join("musicus")
}

pub fn get_playlist_directory() -> PathBuf {
	get_config_directory().join("playlists")
}

pub fn init_config() {
	create_dir(&get_config_directory());
	create_dir(&get_playlist_directory());
}

pub fn load_playlists() -> Vec<Playlist> {
	let playlists_directory = get_playlist_directory();
	let mut playlists = Vec::new();
	for entry in get_dir_entries(&playlists_directory) {
		if entry.is_file {
			playlists.push(Playlist::from_file(&entry.path));
		}
	}
	playlists
}