use crate::render::{Renderable, RenderObject, RenderPanel, RenderEntry, RenderColor};
use std::path::{Path, PathBuf};
use crate::file_utils::{get_dir_entries, DirectoryEntry, get_common_ends};
use crate::musicus::log;

pub struct PlaylistManager {
	current_playlist: usize,
	playlists: Vec<Playlist>,
}

struct Playlist {
	name: String,
	songs: Vec<Song>,
}

pub struct Song {
	title: String,
	path: PathBuf,
}

impl PlaylistManager {
	pub fn new() -> PlaylistManager {
		let mut playlists = Vec::new();
		playlists.push(Playlist::new("my_playlist".to_string()));
		playlists.push(Playlist::new("my_playlist2".to_string()));
		PlaylistManager {
			current_playlist: 0,
			playlists,
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
}

impl Renderable for PlaylistManager {
	fn get_render_object(&self) -> RenderObject {
		let mut render_object = RenderObject::new();

		// add overview panel
		let mut overview_panel = RenderPanel::new(0);
		for playlist in &self.playlists {
			overview_panel.entries.push(RenderEntry::new(playlist.name.clone(), RenderColor::WHITE, RenderColor::BLACK));
		}
		render_object.panels.push(overview_panel);

		// add current playlist
		if let Some(current_playlist) = self.playlists.get(self.current_playlist) {
			let mut panel = RenderPanel::new(0);
			for song in &current_playlist.songs {
				panel.entries.push(RenderEntry::new(song.title.clone(), RenderColor::WHITE, RenderColor::BLACK));
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
		}
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