use crate::audio_backend::{AudioBackend, AudioCommand, AudioInfo, SeekCommand, SeekDirection, AudioBackendCommand};
use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable, RenderColor, RenderPanel, format_duration, Alignment};
use pancurses::{Window, Input};
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashMap;
use crossbeam::{unbounded, Sender, Receiver};
use std::thread;
use crate::playlist_manager::PlaylistManager;
use crate::config::{load_playlists, init_config, get_playlist_directory, Cache, FileManagerCache};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use crate::play_state::{PlayPosition, PlayState, PlayMode};
use crate::debug_manager::DebugManager;
use crate::song::{Song, SongID};
use crate::song::song_buffer::SongBuffer;
use crate::string_helpers::{cut_str_left, limit_str_right};

const FILE_BROWSER_OFFSET: i32 = 5;
const ENTER_CHAR: char = 10 as char;
const CURSES_TIMEOUT: i32 = 200;

pub struct Musicus {
    command_sender: Sender<AudioBackendCommand>,
	info_receiver: Receiver<AudioInfo>,
	file_manager: FileManager,
	playlist_manager: PlaylistManager,
	debug_manager: DebugManager,
	pub song_buffer: SongBuffer,
	window: Window,
	color_pairs: HashMap<(RenderColor, RenderColor), i16>,
	color_pair_counter: i16,
	play_state: PlayState,
	view_state: ViewState,
	playing_song_info: Option<SongInfo>,
	volume: i32,
	follow: bool,
	screen_dimensions: (i32, i32), // height, width
	clipboard: Option<SongID>,
}

struct SongInfo {
	title: String,
	play_position: Duration,
	total_duration: Duration,
	queued_next: bool,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum ViewState {
	FileManager,
	Playlists,
	Debug,
}

impl Musicus {
	pub fn new() -> Musicus {
		// setup config
		init_config();

		let mut debug_manager = DebugManager::new();

		let cache = if let Ok(cache) = Cache::load() {
			cache
		} else {
			debug_manager.add_entry_color("Failed to load cache. Using default.".to_string(), RenderColor::Red, RenderColor::Black);
			Cache::default()
		};

		let song_buffer = if let Ok(song_buffer) = SongBuffer::load() {
			song_buffer
		} else {
			debug_manager.add_entry_color("Failed to load song buffer. Using empty.".to_string(), RenderColor::Red, RenderColor::Black);
			SongBuffer::new()
		};

		// setup curses
		let window = pancurses::initscr();
		Musicus::init_curses(&window);

		// setup audio backend
		let (audio_backend_sender, audio_backend_receiver) = unbounded();
        let (info_sender, info_receiver) = unbounded();

		let audio_backend_sender_clone = audio_backend_sender.clone();

		let backend_volume = cache.volume as f32 * 0.01;

		thread::Builder::new().name("backend".to_string()).spawn(move || {
			let mut audio_backend = AudioBackend::new(info_sender, audio_backend_sender_clone, backend_volume);
			audio_backend.run(audio_backend_receiver);
		}).expect("Failed to spawn backend thread");

		// load playlists
		let playlists = load_playlists();
		let screen_dimensions = window.get_max_yx();

		Musicus {
			command_sender: audio_backend_sender,
            info_receiver,
			file_manager: FileManager::new(&cache.filemanager_cache),
			playlist_manager: PlaylistManager::new(playlists, &cache.playlist_manager_cache),
			debug_manager,
			song_buffer,
			window,
			color_pairs: HashMap::new(),
			color_pair_counter: 1,
			play_state: PlayState::new(cache.play_mode),
			view_state: cache.view,
			playing_song_info: None,
			volume: cache.volume,
			follow: cache.follow,
			screen_dimensions,
			clipboard: None,
		}
	}

	pub fn init_curses(window: &Window) {
		pancurses::noecho();
		pancurses::curs_set(0);
		pancurses::start_color();
		window.timeout(CURSES_TIMEOUT);
		window.keypad(true);
	}

	pub fn shutdown(&mut self) {
		pancurses::endwin();
		let playlists_path = get_playlist_directory();
		for playlist in &self.playlist_manager.playlists {
			let playlist_path = playlists_path.join(playlist.name.to_lowercase().replace(" ", "_")).with_extension("json");
			playlist.dump_to_file(&playlist_path);
		}

		// dump cache
		let cache = Cache {
			view: self.view_state,
			play_mode: self.play_state.mode,
			filemanager_cache: FileManagerCache {
				current_directory: self.file_manager.current_path.clone(),
			},
			playlist_manager_cache: self.playlist_manager.create_cache(),
			volume: self.volume,
			follow: self.follow,
		};
		cache.dump();

		// dump song buffer
		self.song_buffer.dump();
	}

	pub fn run(&mut self) {
		let mut running = true;
		self.render(true);
		while running {
			let got_input = self.handle_input(&mut running);
			let got_update = self.handle_audio_backend();
			let got_log = self.debug_manager.has_update();
			self.render(got_input || got_update || (matches!(self.view_state, ViewState::Debug) && got_log));
		}
		self.shutdown();
	}

	fn start_next_song(&mut self) {
		if let Some(PlayPosition::Playlist(song_id, ..)) = self.play_state.peek_next_song() {
			let song = self.song_buffer.get(song_id).unwrap();
			self.command_sender.send(AudioBackendCommand::Command(AudioCommand::Play(song.clone()))).unwrap();
			if let Err(msg) = self.play_state.play_next_song(&self.playlist_manager) {
				self.debug_manager.add_error_entry(format!("failed to start next song: {}", msg));
			}
			if self.follow {
				self.follow_playlist();
			}
		} else {
			self.debug_manager.add_error_entry("failed to start next song: {}".to_string());
		}
	}

	fn follow_playlist(&mut self) {
		if let Some(PlayPosition::Playlist(_, playlist_index, song_index, false)) = &mut self.play_state.get_current_play_position() { // only match songs, that are not deleted
			self.playlist_manager.set_cursor_position(*playlist_index, *song_index, self.get_num_rows());
		}
	}

	fn get_num_rows(&self) -> usize {
		(self.window.get_max_y()-1) as usize
	}

	fn handle_audio_backend(&mut self) -> bool {
		let mut has_to_render = false;
		let mut should_follow = false;
		for info in self.info_receiver.try_iter() {
			match info {
				AudioInfo::Playing(_song_id, play_position) => {
					if let Some(playing_song) = &mut self.playing_song_info {
						playing_song.play_position = play_position;

						// check for queue command
						if !playing_song.queued_next {
							match self.play_state.peek_next_song() {
								Some(song) => {
									let song = self.song_buffer.get(song.get_id()).unwrap();
									self.command_sender.send(AudioBackendCommand::Command(AudioCommand::Queue(song.clone()))).unwrap();
									self.debug_manager.add_entry(format!("queuing \"{}\"", song.get_title()));
									playing_song.queued_next = true;
								},
								None => self.debug_manager.add_error_entry("failed to peek next song".to_string())
							}
						}
					} else {
						self.debug_manager.add_entry_color("Got playing update, but playing song info is not set.".to_string(), RenderColor::Red, RenderColor::Black);
					}
				}
				AudioInfo::FailedOpen(song_id, e) => {
					self.debug_manager.add_entry_color(
						format!("Failed to open \"{}\": {:?}\n", self.song_buffer.get(song_id).map(|s| s.get_title()).unwrap_or("<unknown song>"), e),
					    RenderColor::Red,
						RenderColor::Black
					);
				}
				AudioInfo::SongStarts(song_id) => {
					if let Some(current_song) = self.play_state.current_song {
						if current_song.get_id() != song_id {
							match self.play_state.play_next_song(&self.playlist_manager) {
								Ok(_) => {}
								Err(_) => self.debug_manager.add_error_entry("failed to start next song".to_string()),
							}
						}
					}
					let song = self.song_buffer.get(song_id).unwrap();
					self.playing_song_info = Some(SongInfo {
						title: song.get_title().to_string(),
						play_position: Duration::new(0, 0),
						total_duration: song.get_total_duration().unwrap_or(Duration::new(0, 0)), // TODO: fix; SongInfo.total_duration should be Option
						queued_next: false,
					});
					has_to_render = true;
					self.debug_manager.add_entry(format!("start song \"{}\"", song.get_title()));
					should_follow = true;
				}
				AudioInfo::SongDuration(song_id, duration) => {
					self.song_buffer.update_total_duration(song_id, duration);
					if matches!(self.view_state, ViewState::Playlists) {
						has_to_render = true;
					}
				}
			}
		}
		if should_follow && self.follow {
			self.follow_playlist();
		}
		has_to_render
	}

	fn handle_input(&mut self, running: &mut bool) -> bool {
		let mut got_valid_input = false;

		if self.screen_dimensions != self.window.get_max_yx() {
			self.screen_dimensions = self.window.get_max_yx();
			got_valid_input = true;
		}
		if let Some(input) = self.window.getch() {
			match input {
				Input::Character(c) => {
					got_valid_input = true;
					match (c, self.view_state) {
						('q', _) => *running = false,
						('L', _) => self.seek(SeekDirection::Forward),
						('H', _) => self.seek(SeekDirection::Backward),
						('J', _) => self.start_next_song(),
						(ENTER_CHAR, ViewState::FileManager) => self.filemanager_context_action(),
						('y', ViewState::FileManager) => self.file_manager_add_to_playlist(),
						('n', ViewState::FileManager) => self.file_manager_new_playlist(),
						('h', ViewState::FileManager) => self.file_manager.move_left(),
						('j', ViewState::FileManager) => self.file_manager.move_down(self.get_num_rows()),
						('k', ViewState::FileManager) => self.file_manager.move_up(),
						('l', ViewState::FileManager) => self.file_manager.move_right(),
						(ENTER_CHAR, ViewState::Playlists) => self.playlist_manager_context_action(),
						('h', ViewState::Playlists) => self.playlist_manager.move_left(),
						('l', ViewState::Playlists) => self.playlist_manager.move_right(),
						('j', ViewState::Playlists) => self.playlist_manager.move_down(self.get_num_rows()),
						('k', ViewState::Playlists) => self.playlist_manager.move_up(self.get_num_rows()),
						('O', ViewState::Playlists) => self.playlist_manager.optimize_names(&mut self.song_buffer),
						('y', ViewState::Playlists) => self.copy_playlist_song_to_clipboard(),
						('p', ViewState::Playlists) => self.paste_clipboard_song_to_playlist(),
						('j', ViewState::Debug) => self.debug_manager.scroll(1),
						('k', ViewState::Debug) => self.debug_manager.scroll(-1),
						('c', _) => self.toggle_pause(),
						('1', _) => self.view_state = ViewState::FileManager,
						('2', _) => self.view_state = ViewState::Playlists,
						('3', _) => self.view_state = ViewState::Debug,
						('s', _) => {
							match self.play_state.toggle_mode(&self.playlist_manager) {
								Err(msg) => self.debug_manager.add_error_entry(format!("Failed to define next song, when toggling mode: {}", msg)),
								_ => {}
							};
							if let Some(playing_song) = &mut self.playing_song_info {
								playing_song.queued_next = false;
							};
						},
						('f', _) => self.follow = !self.follow,
						('F', ViewState::Playlists) => self.follow_playlist(),
						('+', _) => self.change_volume(5),
						('-', _) => self.change_volume(-5),
						('D', ViewState::Playlists) => {
							if self.playlist_manager.get_shown_playlist().is_some() {
								if let Some(shown_song_index) = self.playlist_manager.get_shown_song_index() {
									self.play_state.apply_playlist_delete(self.playlist_manager.shown_playlist_index, shown_song_index);
								}
							}
							self.playlist_manager.delete_current_song();
						},
						('i', ViewState::FileManager) => {
							let errors = self.playlist_manager.import_playlists(&self.file_manager.current_path, &mut self.song_buffer);
							for error in errors {
								self.debug_manager.add_error_entry(error);
							}
						},
						_ => {
							if !matches!(self.view_state, ViewState::Debug) {
								got_valid_input = false;
							}
							self.debug_manager.add_entry(format!("got unknown char: {} ({})\n", c, c as i32));
						},
					}
				}
				_ => {},
			}
		}
		got_valid_input
	}

	fn change_volume(&mut self, volume_change: i32) {
		self.volume = (self.volume + volume_change).clamp(0, 100);
        self.command_sender.send(
			AudioBackendCommand::Command(AudioCommand::SetVolume(self.volume as f32 * 0.01))
		).unwrap();
	}

	fn seek(&mut self, direction: SeekDirection) {
		let duration = Duration::from_secs(5);
		self.command_sender.send(
			AudioBackendCommand::Command(AudioCommand::Seek(SeekCommand {
				duration,
				direction,
			}))
		).unwrap();
		/*
		if let Some(playing_song) = &mut self.playing_song_info {
			playing_song.play_position = (playing_song.play_position + duration).min(playing_song.total_duration) // TODO: use direction here
		}
		 */
	}

	fn toggle_pause(&mut self) {
		if self.play_state.playing {
			self.command_sender.send(AudioBackendCommand::Command(AudioCommand::Pause)).unwrap();
		} else {
			self.command_sender.send(AudioBackendCommand::Command(AudioCommand::Unpause)).unwrap();
		}
		self.play_state.playing = !self.play_state.playing;
	}

	fn filemanager_context_action(&mut self) {
		let song_id = self.song_buffer.import(&self.file_manager.current_path, None);
		let song = self.song_buffer.get(song_id).unwrap();
		Self::play(&self.command_sender, &mut self.play_state, song.clone());
		let _ = self.play_state.play_song(PlayPosition::File(song_id), &self.playlist_manager);
	}

	fn play(command_sender: &Sender<AudioBackendCommand>, play_state: &mut PlayState, song: Song) {
		command_sender.send(AudioBackendCommand::Command(AudioCommand::Play(song))).unwrap();
		play_state.playing = true;
	}

	fn playlist_manager_context_action(&mut self) {
        if let Some(song_id) = self.playlist_manager.get_shown_song() {
			if let Some(song) = self.song_buffer.get(song_id) {
				Self::play(&self.command_sender, &mut self.play_state, song.clone());
				let new_play_position = PlayPosition::Playlist(
					song_id,
					self.playlist_manager.shown_playlist_index,
					self.playlist_manager.get_shown_song_index().unwrap(),
					false,
				);
				let _ = self.play_state.play_song(new_play_position, &self.playlist_manager);
			} else {
				self.debug_manager.add_entry_color(format!("Failed to start song id {}", song_id), RenderColor::Red, RenderColor::Black);
			}
		}
	}

	fn file_manager_add_to_playlist(&mut self) {
		let songs = Song::songs_from_path(&self.file_manager.current_path, &mut self.song_buffer);
		let len_songs = songs.len();
		self.playlist_manager.add_songs(songs);
		if let Some(shown_playlist) = self.playlist_manager.get_shown_playlist() {
			self.debug_manager.add_entry(format!("adding {} songs to playlist \"{}\"", len_songs, shown_playlist.name));
		}
	}

	fn file_manager_new_playlist(&mut self) {
		let songs = Song::songs_from_path(&self.file_manager.current_path, &mut self.song_buffer);
		let name = self.file_manager.current_path.file_name().unwrap().to_str().unwrap().replace(" ", "");

		self.playlist_manager.add_playlist_with_songs(name, songs);
	}

	fn render(&mut self, everything: bool) {
		if everything {
			let render_object = match self.view_state {
				ViewState::FileManager => self.file_manager.get_render_object(),
				ViewState::Playlists => self.playlist_manager.get_render_object(&self.play_state, &self.song_buffer),
				ViewState::Debug => self.debug_manager.get_render_object(),
			};
			self.window.erase();
			self.render_panels(&render_object);
		}
		self.render_play_state();
		self.window.refresh();
	}

	fn render_panels(&mut self, render_object: &RenderObject) {
		let mut x_pos = match render_object.alignment {
			Alignment::Left => 0,
			Alignment::Right => (self.window.get_max_x() - (render_object.get_panels_size() as i32 + render_object.panels.len() as i32*FILE_BROWSER_OFFSET)).min(0),
		};
		for panel in render_object.panels.iter() {
			self.render_panel(panel, x_pos);
			x_pos += panel.get_width() as i32 + FILE_BROWSER_OFFSET;
		}
	}

	fn render_play_state(&mut self) {
		self.set_color(RenderColor::Black, RenderColor::Cyan);
		self.window.mv(self.window.get_max_y() - 1, 0);
		self.window.hline(' ', self.window.get_max_x());
		let playing_str = if self.play_state.playing { ">" } else { "|" };
		let play_mode_str = match self.play_state.mode {
			PlayMode::Normal => " ",
			PlayMode::Shuffle => "S",
		};
		let follow_str = if self.follow { "F" } else { " " };

		let play_state_str = match &self.playing_song_info {
			None => {
				format!(
					"{} {}{}          0:00 / 0:00 vol: {}%",
					playing_str,
					play_mode_str,
					follow_str,
					self.volume,
				)
			}
			Some(current_song) => {
				format!(
					"{} {}{} {}  {} / {}  vol: {}%",
					playing_str,
					play_mode_str,
					follow_str,
					current_song.title,
					format_duration(current_song.play_position),
					format_duration(current_song.total_duration),
					self.volume,
				)
			}
		};
		self.window.mvaddstr(
			self.window.get_max_y()-1,
			1,
			play_state_str,
		);
	}

	fn set_color(&mut self, foreground: RenderColor, background: RenderColor) {
		if let Some(color) = self.color_pairs.get(&(foreground, background)) {
			self.window.color_set(*color);
		} else {
			pancurses::init_pair(self.color_pair_counter, foreground.to_curses_color(), background.to_curses_color());
			self.color_pairs.insert((foreground, background), self.color_pair_counter);
			self.window.color_set(self.color_pair_counter);
			self.color_pair_counter += 1;
		}
	}

	fn render_panel(&mut self, panel: &RenderPanel, x_pos: i32) {
		for (y_pos, e) in panel.entries.iter().skip(panel.scroll_position).take((self.window.get_max_y()-1) as usize).enumerate() {
			self.set_color(e.foreground_color, e.background_color);
			if (e.text.len() as i32) >= -x_pos {
				let line = cut_str_left(&e.text, (-x_pos).max(0) as usize);
				let line = format!("{: <width$}", line, width=panel.get_width() + FILE_BROWSER_OFFSET as usize);
				let line = limit_str_right(&line, ((self.window.get_max_x() - x_pos).max(0) as usize).min(line.len()));
				self.window.mvaddstr(y_pos as i32, x_pos.max(0), line);
			}
		}
	}

	fn copy_playlist_song_to_clipboard(&mut self) {
		self.clipboard = self.playlist_manager.get_shown_song();
	}

	fn paste_clipboard_song_to_playlist(&mut self) {
		if let Some(song_id) = self.clipboard {
			self.playlist_manager.insert_song(song_id);
		}
	}
}

#[allow(unused)]
pub fn log(text: &str) {
	let mut file = OpenOptions::new().write(true).create(true).append(true).open("log.txt").expect("failed to open log file");
	file.write_all(text.as_bytes()).unwrap();
	file.write("\n".as_bytes());
}
