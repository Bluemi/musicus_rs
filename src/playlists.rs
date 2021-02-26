use crate::render::{Renderable, RenderObject, RenderPanel, RenderEntry, RenderColor};
use std::path::{Path, PathBuf};
use crate::file_utils::{get_dir_entries, DirectoryEntry, get_common_ends};
use crate::musicus::log;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use serde::{Serialize, Deserialize};
use crate::config::PlaylistManagerCache;

pub struct PlaylistManager {
	pub current_playlist: usize,
	pub playlists: Vec<Playlist>,
	pub view: PlaylistView,
}

#[derive(Serialize, Deserialize)]
pub struct Playlist {
	pub name: String,
	pub songs: Vec<Song>,
	pub cursor_position: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Song {
	title: String,
	path: PathBuf,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum PlaylistView {
	Overview,
	Playlist,
}

impl PlaylistManager {
	pub fn new(playlists: Vec<Playlist>, cache: &PlaylistManagerCache) -> PlaylistManager {
		PlaylistManager {
			current_playlist: 0,
			playlists,
			view: cache.view
		}
	}

	pub fn add_songs(&mut self, songs: Vec<Song>) {
		if let Some(current_playlist) = self.get_current_playlist() {
			current_playlist.songs.extend(songs);
		}
	}

	fn get_current_playlist(&mut self) -> Option<&mut Playlist> {
		self.playlists.get_mut(self.current_playlist)
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
				if self.current_playlist < self.playlists.len() - 1 {
					self.current_playlist += 1;
				}
			}
			PlaylistView::Playlist => {
				if let Some(playlist) = self.get_current_playlist() {
					if playlist.cursor_position < playlist.songs.len() - 1 {
						playlist.cursor_position += 1;
					}
				}
			}
		}
	}

	pub fn move_up(&mut self) {
		match self.view {
			PlaylistView::Overview => {
				if self.current_playlist > 0 {
					self.current_playlist -= 1;
				}
			}
			PlaylistView::Playlist => {
				if let Some(playlist) = self.get_current_playlist() {
					if playlist.cursor_position > 0 {
						playlist.cursor_position -= 1;
					}
				}
			}
		}
	}
}

impl Renderable for PlaylistManager {
	fn get_render_object(&self) -> RenderObject {
		let mut render_object = RenderObject::new();

		// add overview panel
		let mut overview_panel = RenderPanel::new(0);
		for (index, playlist) in self.playlists.iter().enumerate() {
			let (foreground_color, background_color) = if index == self.current_playlist {
				if matches!(self.view, PlaylistView::Overview) {
					(RenderColor::WHITE, RenderColor::BLUE)
				} else {
					(RenderColor::BLACK, RenderColor::WHITE)
				}
			} else {
				(RenderColor::WHITE, RenderColor::BLACK)
			};
			overview_panel.entries.push(RenderEntry::new(playlist.name.clone(), foreground_color, background_color));
		}
		render_object.panels.push(overview_panel);

		// add current playlist
		if let Some(current_playlist) = self.playlists.get(self.current_playlist) {
			let mut panel = RenderPanel::new(0);
			for (index, song) in current_playlist.songs.iter().enumerate() {
				let (foreground_color, background_color) = if index == current_playlist.cursor_position {
					if matches!(self.view, PlaylistView::Playlist) {
						(RenderColor::WHITE, RenderColor::BLUE)
					} else {
						(RenderColor::BLACK, RenderColor::WHITE)
					}
				} else {
					(RenderColor::WHITE, RenderColor::BLACK)
				};
				panel.entries.push(RenderEntry::new(song.title.clone(), foreground_color, background_color));
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

		let files = sound_files.iter().map(|de| &*de.filename).collect::<Vec<&str>>().join("\n\t");
		log(&format!("common begin: \"{}\"\tcommon end: \"{}\"\n\t{}\n\n", start, end, files));

		let mut songs = Vec::new();

		for sound_file in sound_files {
			let title = &sound_file.filename[start.len()..sound_file.filename.len()-end.len()];
			songs.push(
				Song {
					title: title.to_string(),
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