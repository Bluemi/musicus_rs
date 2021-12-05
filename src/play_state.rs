use rand::random;
use serde::{Serialize, Deserialize};
use crate::playlist_manager::PlaylistManager;
use crate::song::SongID;

pub struct PlayState {
	pub playing: bool,
	pub mode: PlayMode,
	pub history: Vec<PlayPosition>,
	pub current_song: Option<PlayPosition>,
	pub next_song: Option<PlayPosition>,
}

impl PlayState {
	pub fn new(mode: PlayMode) -> PlayState {
		PlayState {
			playing: false,
			mode,
			history: Vec::new(),
			current_song: None,
			next_song: None,
		}
	}

	pub fn is_playlist_played(&self, playlist_index: usize) -> bool {
		if let Some(PlayPosition::Playlist(_, playlist, ..)) = self.get_current_play_position() {
			playlist_index == playlist
		} else {
			false
		}
	}

	pub fn is_song_played(&self, playlist_index: usize, song_index: usize) -> bool {
		if let Some(PlayPosition::Playlist(_, playlist, song, ..)) = self.get_current_play_position() {
			playlist_index == playlist && song_index == song
		} else {
			false
		}
	}

	pub fn play_song(&mut self, play_position: PlayPosition, playlist_manager: &PlaylistManager) -> Result<(), String>{
		self.history.push(play_position);
		self.current_song = Some(play_position);
		self.define_next_song(playlist_manager).map(|_| ())
	}

	/**
	 * Gets the current play position
	 */
	pub fn get_current_play_position(&self) -> Option<PlayPosition> {
		self.current_song
	}

	/**
	 * Peeks the next song (and defines it, if not already defined).
	 */
	pub fn peek_next_song(&mut self) -> Option<PlayPosition> {
		self.next_song
	}

	/**
	 * Changes the current play position to the next song.
	 */
	pub fn play_next_song(&mut self, playlist_manager: &PlaylistManager) -> Result<(), String>{
		if let Some(next_song) = self.next_song {
			self.play_song(next_song, playlist_manager)
		} else {
			Err("Cannot play next song: no next song".to_string())
		}
	}

	/**
	 * If necessary generates the next play position and writes it into the history.
	 * It is impossible to generate next songs for PlayState::Empty, PlayState::File or end of playlist.
	 */
	pub fn define_next_song(&mut self, playlist_manager: &PlaylistManager) -> Result<PlayPosition, String> {
		match PlayState::generate_next_song(&self.mode, &self.current_song.ok_or("no current song".to_string())?, playlist_manager) {
			Ok((song_id, song_index, playlist_index)) => {
				let next_play_position = PlayPosition::Playlist(song_id, playlist_index, song_index, false);
				self.set_next_song(next_play_position);
				Ok(next_play_position)
			},
			Err(msg) => {
				Err(msg)
			},
		}
	}

	fn set_next_song(&mut self, play_position: PlayPosition) {
		self.next_song = Some(play_position);
	}

	/**
	 * Generates a possible next song that should be played. Does not write into history.
	 */
	fn generate_next_song(mode: &PlayMode, play_position: &PlayPosition, playlist_manager: &PlaylistManager) -> Result<(SongID, usize, usize), String> {
		match play_position {
			PlayPosition::Playlist(_song_id, playlist_index, song_index, ..) => {
				let next_song_index = match mode {
					PlayMode::Normal => *song_index + 1,
					PlayMode::Shuffle => {
						let played_playlist = playlist_manager.playlists.get(*playlist_index).unwrap();
						random::<usize>() % played_playlist.songs.len()
					},
				};
				let song_id = playlist_manager.get_song(*playlist_index, next_song_index)
					.ok_or(format!("get song failed with playlist_index={} song_index={}", *playlist_index, next_song_index))?;
				Ok((song_id, next_song_index, *playlist_index))
			}
			PlayPosition::File(_) => Err("file".to_string()),
		}
	}

	pub fn toggle_mode(&mut self, playlist_manager: &PlaylistManager) -> Result<(), String>{
		self.mode = match self.mode {
			PlayMode::Normal => PlayMode::Shuffle,
			PlayMode::Shuffle => PlayMode::Normal,
		};
		self.define_next_song(playlist_manager).map(|_| ())
	}

	pub fn apply_playlist_delete(&mut self, arg_playlist_id: usize, arg_song_index: usize) {
		if let Some(current_song) = &mut self.current_song {
			current_song.apply_playlist_delete(arg_playlist_id, arg_song_index);
		}
		if let Some(next_song) = &mut self.next_song {
			next_song.apply_playlist_delete(arg_playlist_id, arg_song_index);
		}
		for song in self.history.iter_mut() {
			song.apply_playlist_delete(arg_playlist_id, arg_song_index);
		}
	}
}

#[derive(Copy, Clone)]
pub enum PlayPosition {
	File(SongID), // A Song from the file browser was played
	Playlist(SongID, usize, usize, bool), // (song_id, playlist_id, song_index in playlist, deleted)
}

impl PlayPosition {
	/**
	 * If a song of a playlist is deleted, we have to adjust song_index of songs later in the playlist and the state of the deleted song
	 */
	fn apply_playlist_delete(&mut self, arg_playlist_id: usize, arg_song_index: usize) {
		if let PlayPosition::Playlist(_, playlist_id, song_index, deleted) = self {
			if *playlist_id == arg_playlist_id {
				if *song_index == arg_song_index { // this play position is deleted
					*deleted = true;
				} else if *song_index > arg_song_index { // a song before this song was deleted
					*song_index -= 1; // we decrease it
				}
			}
		}
	}
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum PlayMode {
	Normal,
	Shuffle,
}
