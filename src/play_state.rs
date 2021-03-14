use std::path::PathBuf;
use crate::playlists::{PlaylistManager, Song};
use random::Source;

pub struct PlayState {
	pub playing: bool,
	pub play_position: PlayPosition,
	pub mode: PlayMode,
	random_source: random::Default,
}

impl PlayState {
	pub fn new() -> PlayState {
		PlayState {
			playing: false,
			play_position: PlayPosition::Empty,
			mode: PlayMode::Normal,
			random_source: random::default(),
		}
	}

	pub fn is_playlist_played(&self, playlist_index: usize) -> bool {
		if let PlayPosition::Playlist(playlist, _) = self.play_position {
			playlist_index == playlist
		} else {
			false
		}
	}

	pub fn is_song_played(&self, playlist_index: usize, song_index: usize) -> bool {
		if let PlayPosition::Playlist(playlist, song) = self.play_position {
			playlist_index == playlist && song_index == song
		} else {
			false
		}
	}

	pub fn get_next_song(&mut self, playlist_manager: &mut PlaylistManager) -> Option<Song> {
		match &mut self.play_position {
			PlayPosition::Playlist(playlist_index, song_index) => {
				*song_index = match self.mode {
					PlayMode::Normal => *song_index + 1,
					PlayMode::Shuffle => {
						let played_playlist = playlist_manager.playlists.get(*playlist_index).unwrap();
						self.random_source.read::<usize>() % played_playlist.songs.len()
					},
				};
				playlist_manager.get_song(*playlist_index, *song_index).map(|s| s.clone())
			}
			_ => None,
		}
	}

	pub fn toggle_mode(&mut self) {
		self.mode = match self.mode {
			PlayMode::Normal => PlayMode::Shuffle,
			PlayMode::Shuffle => PlayMode::Normal,
		}
	}
}

pub enum PlayPosition {
	Empty,
	File(PathBuf),
	Playlist(usize, usize), // playlist index, song index
}

pub enum PlayMode {
	Normal,
	Shuffle,
}
