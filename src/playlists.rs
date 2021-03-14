use crate::render::{RenderObject, RenderPanel, RenderEntry, RenderColor};
use std::path::{Path, PathBuf};
use crate::file_manager::file_utils::{get_dir_entries, DirectoryEntry, get_common_ends};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use serde::{Serialize, Deserialize};
use crate::config::PlaylistManagerCache;
use crate::play_state::PlayState;

pub struct PlaylistManager {
	pub shown_playlist_index: usize,
	pub playlists: Vec<Playlist>,
	pub view: PlaylistView,
	pub num_rows: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Playlist {
	pub name: String,
	pub songs: Vec<Song>,
	pub cursor_position: usize,
	pub scroll_position: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Song {
	pub title: String,
	pub path: PathBuf,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum PlaylistView {
	Overview,
	Playlist,
}

impl PlaylistManager {
	pub fn new(playlists: Vec<Playlist>, cache: &PlaylistManagerCache, num_rows: usize) -> PlaylistManager {
		PlaylistManager {
			shown_playlist_index: 0,
			playlists,
			view: cache.view,
			num_rows,
		}
	}

	pub fn add_songs(&mut self, songs: Vec<Song>) {
		if let Some(shown_playlist) = self.get_shown_playlist() {
			shown_playlist.songs.extend(songs);
		}
	}

	pub fn get_shown_playlist(&mut self) -> Option<&mut Playlist> {
		self.playlists.get_mut(self.shown_playlist_index)
	}

	pub fn get_shown_song(&mut self) -> Option<&mut Song> {
		if let Some(shown_playlist) = self.get_shown_playlist() {
			return shown_playlist.songs.get_mut(shown_playlist.cursor_position);
		}
		None
	}

	pub fn get_song(&mut self, playlist_index: usize, song_index: usize) -> Option<&mut Song> {
		if let Some(playlist) = self.playlists.get_mut(playlist_index) {
			if let Some(song) = playlist.songs.get_mut(song_index) {
				return Some(song);
			}
		}
		None
	}

	pub fn move_left(&mut self) {
		self.view = PlaylistView::Overview;
	}

	pub fn move_right(&mut self) {
		self.view = PlaylistView::Playlist;
	}

	pub fn move_down(&mut self) {
        let num_rows = self.num_rows as i32;
		match self.view {
			PlaylistView::Overview => {
				if self.shown_playlist_index < self.playlists.len() - 1 {
					self.shown_playlist_index += 1;
				}
			}
			PlaylistView::Playlist => {
				if let Some(playlist) = self.get_shown_playlist() {
					if playlist.cursor_position < playlist.songs.len() - 1 {
						playlist.cursor_position += 1;
						playlist.scroll_position = (playlist.scroll_position as i32).max(playlist.cursor_position as i32-num_rows + 1) as usize;
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
				if let Some(playlist) = self.get_shown_playlist() {
					if playlist.cursor_position > 0 {
						playlist.cursor_position -= 1;
						if playlist.scroll_position > playlist.cursor_position {
							playlist.scroll_position = playlist.cursor_position;
						}
					}
				}
			}
		}
	}

	pub fn get_render_object(&self, play_state: &PlayState) -> RenderObject {
		let mut render_object = RenderObject::new();

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
			for (index, song) in playlist.songs.iter().enumerate() {
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
				panel.entries.push(RenderEntry::new(song.title.clone(), foreground_color, background_color));
                panel.scroll_position = playlist.scroll_position;
			}
			render_object.panels.push(panel);
		}

		render_object
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
}

impl Song {
	pub fn songs_from_path(path: &Path) -> Vec<Song> {
		let dir_entries = get_dir_entries(path);
		let sound_files: Vec<&DirectoryEntry> = dir_entries.iter().filter(|de| de.is_song_file()).collect();
		let sub_directories: Vec<&DirectoryEntry> = dir_entries.iter().filter(|de| !de.is_file).collect();

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
		}

		for sub_directory in sub_directories {
			songs.extend(Song::songs_from_path(&sub_directory.path));
		}

		songs
	}
}

fn normalize_title(title: &str, index: usize) -> String {
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