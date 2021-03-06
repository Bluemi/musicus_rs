use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, BufRead};
use serde::{Serialize, Deserialize};

use crate::render::{RenderObject, RenderPanel, RenderEntry, RenderColor, Alignment, format_duration};
use crate::file_manager::file_utils::get_dir_entries;
use crate::config::PlaylistManagerCache;
use crate::play_state::PlayState;
use crate::song::SongID;
use crate::song::song_buffer::SongBuffer;
use crate::song::playlist::{Playlist, PlaylistID};
use std::collections::HashMap;

pub struct PlaylistManager {
	pub shown_playlist_index: usize,
	pub playlists: Vec<Playlist>,
	pub view: PlaylistView,
	scroll_cursor_positions: HashMap<PlaylistID, (usize, usize)>,
}


#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum PlaylistView {
	Overview,
	Playlist,
}

impl PlaylistManager {
	pub fn new(playlists: Vec<Playlist>, cache: &PlaylistManagerCache) -> PlaylistManager {
		PlaylistManager {
			shown_playlist_index: cache.shown_playlist_index,
			playlists,
			view: cache.view,
			scroll_cursor_positions: cache.scroll_cursor_positions.clone(),
		}
	}

	pub fn create_cache(&self) -> PlaylistManagerCache {
		PlaylistManagerCache {
			view: self.view,
			shown_playlist_index: self.shown_playlist_index,
			scroll_cursor_positions: self.scroll_cursor_positions.clone(),
		}
	}

	pub fn add_songs(&mut self, songs: Vec<SongID>) {
		if let Some(shown_playlist) = self.get_mut_shown_playlist() {
			shown_playlist.songs.extend(songs);
		}
	}

	pub fn get_mut_shown_playlist(&mut self) -> Option<&mut Playlist> {
		self.playlists.get_mut(self.shown_playlist_index)
	}

	pub fn get_shown_playlist(&self) -> Option<&Playlist> {
		self.playlists.get(self.shown_playlist_index)
	}

	pub fn get_shown_song(&mut self) -> Option<SongID> {
		if let Some(shown_playlist) = self.get_shown_playlist() {
			let cursor_position = self.scroll_cursor_positions.get(&shown_playlist.id).map_or(0, |(_s, c)| *c);
			return shown_playlist.songs.get(cursor_position).map(|s| *s);
		}
		None
	}

	pub fn get_shown_song_index(&mut self) -> Option<usize> {
		Some(self.scroll_cursor_positions.get(&self.get_shown_playlist()?.id).map(|(_s, c)| *c).unwrap_or(0))
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

	pub fn move_down(&mut self, num_rows: usize) {
		match self.view {
			PlaylistView::Overview => {
				if self.shown_playlist_index < self.playlists.len() - 1 {
					self.shown_playlist_index += 1;
				}
			}
			PlaylistView::Playlist => {
				if let Some(playlist_id) = self.get_mut_shown_playlist().map(|p| p.id) {
					let cursor_pos = self.scroll_cursor_positions.get(&playlist_id).map_or(0, |(_s, c)| *c);
					self.set_cursor_position(self.shown_playlist_index, cursor_pos+1, num_rows);
				}
			}
		}
	}

	pub fn move_up(&mut self, num_rows: usize) {
		match self.view {
			PlaylistView::Overview => {
				if self.shown_playlist_index > 0 {
					self.shown_playlist_index -= 1;
				}
			}
			PlaylistView::Playlist => {
				if let Some(playlist_id) = self.get_mut_shown_playlist().map(|p| p.id) {
					let cursor_pos = self.scroll_cursor_positions.get(&playlist_id).map_or(0, |(_s, c)| *c);
					if cursor_pos > 0 {
						self.set_cursor_position(self.shown_playlist_index, cursor_pos - 1, num_rows);
					}
				}
			}
		}
	}

	pub fn set_cursor_position(&mut self, playlist_index: usize, cursor: usize, num_rows: usize) {
		if let Some(playlist) = self.playlists.get(playlist_index) {
			let cursor = cursor.min(playlist.songs.len().checked_sub(1).unwrap_or(0)) as i32;
			let scroll = self.scroll_cursor_positions.get(&playlist.id).map_or(0, |(s, _c)| *s) as i32;
			let scroll = scroll.clamp(
				cursor - num_rows as i32 + 1,
				cursor
			) as usize;
			self.scroll_cursor_positions.insert(playlist.id, (scroll, cursor as usize));
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
			let mut songs_panel = RenderPanel::new(0);
			let mut duration_panel = RenderPanel::new(0);
			let (scroll_position, cursor_position) = self.scroll_cursor_positions.get(&playlist.id).map(|(s, c)| (*s, *c)).unwrap_or((0, 0));
			for (index, song_id) in playlist.songs.iter().enumerate() {
				let (foreground_color, background_color) = if play_state.is_song_played(self.shown_playlist_index, index) {
					if index == cursor_position {
						if matches!(self.view, PlaylistView::Playlist) {
							(RenderColor::YELLOW, RenderColor::BLUE)
						} else {
							(RenderColor::YELLOW, RenderColor::WHITE)
						}
					} else {
						(RenderColor::YELLOW, RenderColor::BLACK)
					}
				} else {
					if index == cursor_position {
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
				songs_panel.entries.push(RenderEntry::new(
					song.get_title().to_string(),
					foreground_color,
					background_color
				));
				duration_panel.entries.push(RenderEntry::new(
					song.get_total_duration().map_or("".to_string(), |d| format_duration(d)),
					foreground_color,
					background_color,
				));
			}
			songs_panel.scroll_position = scroll_position;
			duration_panel.scroll_position = scroll_position;
			render_object.panels.push(songs_panel);
			render_object.panels.push(duration_panel);
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

	pub fn import_playlists(&mut self, path: &Path, song_buffer: &mut SongBuffer) -> Vec<String> {
		let mut errors = Vec::new();
		if path.is_file() {
			// TODO: extract method
			match PlaylistManager::try_import_playlist_file(&path) {
				Ok(paths) => {
					self.add_playlist_from_files(&paths, &path, song_buffer);
				}
				Err(e) => errors.push(format!("error importing playlist file: {}", e))
			}
		} else {
			for entry in get_dir_entries(path) {
				if entry.is_file {
					if let Ok(paths) = PlaylistManager::try_import_playlist_file(&entry.path) {
						self.add_playlist_from_files(&paths, &entry.path, song_buffer);
					}
				} else {
					errors.extend(self.import_playlists(&entry.path, song_buffer));
				}
			}
		}
		errors
	}

	fn add_playlist_from_files(&mut self, paths: &Vec<PathBuf>, path: &Path, song_buffer: &mut SongBuffer) -> PlaylistID {
		let mut songs = Vec::new();
		for path in paths {
			let id = song_buffer.import(path, None);
			songs.push(id);
		}
		self.add_playlist_with_songs(path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or("<no-name>".to_string()), songs)
	}

	fn get_next_playlist_id(&self) -> PlaylistID {
		let mut playlist_id: PlaylistID = 0;
		loop {
			for playlist in &self.playlists {
				if playlist.id == playlist_id {
					playlist_id += 1;
					break;
				}
			}
			return playlist_id;
		}
	}

	pub fn add_playlist_with_songs(&mut self, name: String, songs: Vec<SongID>) -> PlaylistID {
		let id = self.get_next_playlist_id();
		self.playlists.push(Playlist {
			id,
			name,
			songs,
		});
		id
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
	use crate::playlist_manager::normalize_title;

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