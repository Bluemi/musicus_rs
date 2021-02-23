use std::path::PathBuf;
use crate::file_utils::{create_dir, get_dir_entries};
use crate::playlists::{Playlist, PlaylistView};
use serde::{Serialize, Deserialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::env::current_dir;

pub fn get_config_directory() -> PathBuf {
	dirs::config_dir().unwrap().join("musicus")
}

pub fn get_playlist_directory() -> PathBuf {
	get_config_directory().join("playlists")
}

pub fn get_cache_path() -> PathBuf {
	get_config_directory().join("cache.json")
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

#[derive(Serialize, Deserialize)]
pub struct Cache {
	pub filemanager_cache: FileManagerCache,
	pub playlist_manager_cache: PlaylistManagerCache,
}

#[derive(Serialize, Deserialize)]
pub struct FileManagerCache {
	pub current_directory: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct PlaylistManagerCache {
	pub view: PlaylistView,
}

impl Cache {
	pub fn load() -> Cache {
		let cache_path = get_cache_path();
		if cache_path.is_file() {
			let file = File::open(cache_path).unwrap();
			let reader = BufReader::new(file);
			serde_json::from_reader(reader).unwrap()
		} else {
			Cache::default()
		}
	}

	pub fn dump(&self) {
		let file = OpenOptions::new()
			.write(true)
			.truncate(true)
			.create(true)
			.open(get_cache_path())
			.unwrap();
		let writer = BufWriter::new(file);
		serde_json::to_writer(writer, &self).unwrap();
	}

	pub fn default() -> Cache {
		Cache {
			filemanager_cache: FileManagerCache {
				current_directory: current_dir().unwrap_or(PathBuf::new()),
			},
			playlist_manager_cache: PlaylistManagerCache {
				view: PlaylistView::Overview,
			}
		}
	}
}