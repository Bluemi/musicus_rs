use crate::render::{RenderObject, RenderPanel, RenderEntry, RenderColor, Alignment};
use std::path::{Path, PathBuf};
use crate::file_manager::file_utils::get_dir_entries;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, BufRead};
use crate::config::PlaylistManagerCache;
use serde::{Serialize, Deserialize};
use crate::play_state::PlayState;
use crate::musicus::log;
use crate::song::SongID;
use crate::song::song_buffer::SongBuffer;

pub struct PlaylistManager {
	pub shown_playlist_index: usize,
	pub playlists: Vec<Playlist>,
	pub view: PlaylistView,
	pub num_rows: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Playlist {
	pub name: String,
	pub songs: Vec<SongID>,
	pub cursor_position: usize,
	pub scroll_position: usize,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum PlaylistView {
	Overview,
	Playlist,
}

impl PlaylistManager {
	pub fn new(playlists: Vec<Playlist>, cache: &PlaylistManagerCache, num_rows: usize) -> PlaylistManager {
		PlaylistManager {
			shown_playlist_index: cache.shown_playlist_index,
			playlists,
			view: cache.view,
			num_rows,
		}
	}

	pub fn create_cache(&self) -> PlaylistManagerCache {
		PlaylistManagerCache {
			view: self.view,
			shown_playlist_index: self.shown_playlist_index,
		}
	}

	pub fn add_songs(&mut self, songs: Vec<SongID>) {
		if let Some(shown_playlist) = self.get_shown_playlist() {
			shown_playlist.songs.extend(songs);
		}
	}

	pub fn get_shown_playlist(&mut self) -> Option<&mut Playlist> {
		self.playlists.get_mut(self.shown_playlist_index)
	}

	pub fn get_shown_song(&mut self) -> Option<SongID> {
		if let Some(shown_playlist) = self.get_shown_playlist() {
			return shown_playlist.songs.get(shown_playlist.cursor_position).map(|s| *s);
		}
		None
	}

	pub fn get_song(&self, playlist_index: usize, song_index: usize) -> Option<SongID> {
		self.playlists.get(playlist_index)?.songs.get(song_index).map(|s| *s)
	}

	pub fn move_left(&mut self) {
		self.view = PlaylistView::Overview;
	}

	pub fn move_right(&mut self) {
		self.view = PlaylistView::Playlist;
	}

	pub fn move_down(&mut self) {
		match self.view {
			PlaylistView::Overview => {
				if self.shown_playlist_index < self.playlists.len() - 1 {
					self.shown_playlist_index += 1;
				}
			}
			PlaylistView::Playlist => {
				let num_rows = self.num_rows;
				if let Some(playlist) = self.get_shown_playlist() {
					if playlist.cursor_position < playlist.songs.len() - 1 {
						playlist.cursor_position += 1;
						playlist.normalize_scroll_position(num_rows);
					}
				}
			}
		}
	}

	pub fn move_up(&mut self) {
		match self.view {
			PlaylistView::Overview => {
				if self.shown_playlist_index > 0 {
					self.shown_playlist_index -= 1;
				}
			}
			PlaylistView::Playlist => {
				let num_rows = self.num_rows;
				if let Some(playlist) = self.get_shown_playlist() {
					if playlist.cursor_position > 0 {
						playlist.cursor_position -= 1;
						playlist.normalize_scroll_position(num_rows);
					}
				}
			}
		}
	}

	pub fn get_render_object(&self, play_state: &PlayState, song_buffer: &SongBuffer) -> RenderObject {
		let mut render_object = RenderObject::new(Alignment::Left);

		// add overview panel
		let mut overview_panel = RenderPanel::new(0);
		for (index, playlist) in self.playlists.iter().enumerate() {
			let (foreground_color, background_color) = if play_state.is_playlist_played(index) {
				if index == self.shown_playlist_index {
					if matches!(self.view, PlaylistView::Overview) {
						(RenderColor::YELLOW, RenderColor::BLUE)
					} else {
						(RenderColor::YELLOW, RenderColor::WHITE)
					}
				} else {
					(RenderColor::YELLOW, RenderColor::BLACK)
				}
			} else {
				if index == self.shown_playlist_index {
					if matches!(self.view, PlaylistView::Overview) {
						(RenderColor::WHITE, RenderColor::BLUE)
					} else {
						(RenderColor::BLACK, RenderColor::WHITE)
					}
				} else {
					(RenderColor::WHITE, RenderColor::BLACK)
				}
			};

			overview_panel.entries.push(RenderEntry::new(playlist.name.clone(), foreground_color, background_color));
		}
		render_object.panels.push(overview_panel);

		// add shown playlist
		if let Some(playlist) = self.playlists.get(self.shown_playlist_index) {
			let mut panel = RenderPanel::new(0);
			for (index, song_id) in playlist.songs.iter().enumerate() {
				let (foreground_color, background_color) = if play_state.is_song_played(self.shown_playlist_index, index) {
					if index == playlist.cursor_position {
						if matches!(self.view, PlaylistView::Playlist) {
							(RenderColor::YELLOW, RenderColor::BLUE)
						} else {
							(RenderColor::YELLOW, RenderColor::WHITE)
						}
					} else {
						(RenderColor::YELLOW, RenderColor::BLACK)
					}
				} else {
					if index == playlist.cursor_position {
						if matches!(self.view, PlaylistView::Playlist) {
							(RenderColor::WHITE, RenderColor::BLUE)
						} else {
							(RenderColor::BLACK, RenderColor::WHITE)
						}
					} else {
						(RenderColor::WHITE, RenderColor::BLACK)
					}
				};
				let song = song_buffer.get(*song_id).unwrap();
				panel.entries.push(RenderEntry::new(song.get_title().to_string(), foreground_color, background_color));
			}
			panel.scroll_position = playlist.scroll_position;
			render_object.panels.push(panel);
		}

		render_object
	}

	pub fn try_import_playlist_file(path: &Path) -> Result<Vec<PathBuf>, String> {
		if path.is_file() {
			if let Ok(file) = File::open(path) {
				let mut files = Vec::new();
				let mut reader = BufReader::new(file);
				let mut line = String::new();
				loop {
					match reader.read_line(&mut line) {
						Ok(bytes_read) => {
							if bytes_read == 0 {
								return Ok(files);
							}
							let path = PathBuf::from(&line.trim());
							if path.is_file() {
								files.push(path);
							}
							line.clear();
						}
						Err(_) => {
							return Err(format!("Could read file \"{:?}\"", path));
						}
					}
				}
			} else {
				return Err(format!("Failed to open \"{:?}\"", path));
			}
		} else {
			return Err(format!("Cant import \"{:?}\" as it is not a file", path));
		}
	}

	pub fn import_playlists(&mut self, path: &Path, song_buffer: &mut SongBuffer) {
		if path.is_file() {
			// TODO: extract method
			match PlaylistManager::try_import_playlist_file(&path) {
				Ok(paths) => {
					log(&format!("paths: {:?}\n", paths));
					let playlist = PlaylistManager::playlist_from_files(&paths, &path, song_buffer);
					self.playlists.push(playlist);
				}
				Err(e) => log(&format!("error importing playlist file: {}", e))
			}
		} else {
			for entry in get_dir_entries(path) {
				if entry.is_file {
					if let Ok(paths) = PlaylistManager::try_import_playlist_file(&entry.path) {
						let playlist = PlaylistManager::playlist_from_files(&paths, &entry.path, song_buffer);
						self.playlists.push(playlist);
					}
				} else {
					self.import_playlists(&entry.path, song_buffer);
				}
			}
		}
	}

	fn playlist_from_files(paths: &Vec<PathBuf>, path: &Path, song_buffer: &mut SongBuffer) -> Playlist {
		let mut songs = Vec::new();
		for path in paths {
			let id = song_buffer.import(path, None);
			songs.push(id);
		}
		Playlist {
			name: path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or("<no-name>".to_string()),
			songs,
			cursor_position: 0,
			scroll_position: 0
		}
	}
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

pub fn normalize_title(title: &str, index: usize) -> String {
	enum State {
		Init,
		Number(u32),
		Other,
	}
	// check for numbers at title begin
    let mut state = State::Init;

	for c in title.chars() {
        if c.is_digit(10) {
			state = match state {
				State::Init => State::Number(1),
				State::Number(l) => State::Number(l+1),
				State::Other => unreachable!("Failed to normalize title. State::Other should never occur"),
			}
		} else if c == ' ' {
			state = match state {
				State::Init => State::Other,
				State::Number(_) => break,
				State::Other => unreachable!("Failed to normalize title. State::Other should never occur"),
			}
		} else {
			state = match state {
				State::Init => State::Other,
				State::Number(_) => State::Other, // Number directly followed by letter is counted as word
				State::Other => unreachable!("Failed to normalize title. State::Other should never occur"),
			}
		}
        if matches!(state, State::Other) {
			break;
		}
	}

	match state {
		State::Init => "<no title>".to_string(),
		State::Number(1) => format!("0{}", title),
		State::Number(2) => title.to_string(),
		State::Number(_) => format!("{:02} {}", index, title),
		State::Other => format!("{:02} {}", index, title),
	}
}

mod tests {
    #[allow(unused_imports)]
	use crate::playlists::normalize_title;

	#[test]
	fn test_normalize_title1() {
		let title = "1 Heyhey";
		let normalized_title = normalize_title(title, 0);
		assert_eq!(&normalized_title, "01 Heyhey")
	}

	#[test]
	fn test_normalize_title2() {
		let title = "1Heyhey";
		let normalized_title = normalize_title(title, 1);
		assert_eq!(&normalized_title, "01 1Heyhey")
	}
}