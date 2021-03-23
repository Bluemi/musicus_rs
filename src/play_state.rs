use crate::playlists::PlaylistManager;
use crate::random::RandomGenerator;
use crate::song::Song;

pub struct PlayState {
	pub playing: bool,
	pub play_position: PlayPosition,
	pub mode: PlayMode,
	random_generator: RandomGenerator,
}

impl PlayState {
	pub fn new() -> PlayState {
		PlayState {
			playing: false,
			play_position: PlayPosition::Empty,
			mode: PlayMode::Normal,
			random_generator: RandomGenerator::new(),
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

	pub fn get_next_song<'a>(&mut self, playlist_manager: &'a PlaylistManager) -> Option<&'a Song> {
		match &mut self.play_position {
			PlayPosition::Playlist(playlist_index, song_index) => {
				let next_song_index = match self.mode {
					PlayMode::Normal => *song_index + 1,
					PlayMode::Shuffle => {
						let played_playlist = playlist_manager.playlists.get(*playlist_index).unwrap();
						self.random_generator.get_offset_unchecked(1) % played_playlist.songs.len()
					},
				};
				playlist_manager.get_song(*playlist_index, next_song_index)
			}
			_ => None,
		}
	}

	pub fn next_song(&mut self, playlist_manager: &PlaylistManager) {
		if let PlayPosition::Playlist(playlist_index, song_index) = &mut self.play_position {
			match self.mode {
				PlayMode::Normal => *song_index += 1,
				PlayMode::Shuffle => {
					let played_playlist = playlist_manager.playlists.get(*playlist_index).unwrap();
					*song_index = self.random_generator.next() % played_playlist.songs.len();
				},
			};
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
	File(Song),
	Playlist(usize, usize), // playlist index, song index
}

pub enum PlayMode {
	Normal,
	Shuffle,
}
