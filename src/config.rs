use std::path::PathBuf;
use crate::file_manager::file_utils::{create_dir, get_dir_entries};
use crate::playlist_manager::PlaylistView;
use serde::{Serialize, Deserialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::env::current_dir;
use crate::musicus::ViewState;
use crate::song::playlist::{Playlist, PlaylistID};
use std::collections::HashMap;
use crate::play_state::PlayMode;

pub fn get_config_directory() -> PathBuf {
	dirs::config_dir().unwrap().join("musicus")
}

pub fn get_playlist_directory() -> PathBuf {
	get_config_directory().join("playlists")
}

pub fn get_cache_path() -> PathBuf {
	get_config_directory().join("cache.json")
}

pub fn get_song_buffer_path() -> PathBuf {
	get_config_directory().join("lib.json")
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
			if let Ok(playlist) = Playlist::from_file(&entry.path) {
				playlists.push(playlist);
			}
		}
	}
	playlists
}

#[derive(Serialize, Deserialize)]
pub struct Cache {
	pub view: ViewState,
	pub play_mode: PlayMode,
	pub filemanager_cache: FileManagerCache,
	pub playlist_manager_cache: PlaylistManagerCache,
	pub volume: i32,
	pub follow: bool,
}

#[derive(Serialize, Deserialize)]
pub struct FileManagerCache {
	pub current_directory: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct PlaylistManagerCache {
	pub view: PlaylistView,
	pub shown_playlist_index: usize,
	pub playlist_scroll_position: usize,
	pub scroll_cursor_positions: HashMap<PlaylistID, (usize, usize)>
}

impl Cache {
	pub fn load() -> Result<Cache, ()> {
		let cache_path = get_cache_path();
		if cache_path.is_file() {
			let file = File::open(cache_path).unwrap();
			let reader = BufReader::new(file);
			if let Ok(cache) = serde_json::from_reader(reader) {
				Ok(cache)
			} else {
				Err(())
			}
		} else {
			Ok(Cache::default())
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
		serde_json::to_writer_pretty(writer, &self).unwrap();
	}

	pub fn default() -> Cache {
		Cache {
			view: ViewState::FileManager,
			play_mode: PlayMode::Normal,
			filemanager_cache: FileManagerCache {
				current_directory: current_dir().unwrap_or_default(),
			},
			playlist_manager_cache: PlaylistManagerCache {
				view: PlaylistView::Overview,
				playlist_scroll_position: 0,
				shown_playlist_index: 0,
				scroll_cursor_positions: HashMap::new(),
			},
			volume: 100,
			follow: true,
		}
	}
}