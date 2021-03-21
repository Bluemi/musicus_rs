use crate::audio_backend::{AudioBackend, AudioCommand, AudioInfo, SeekCommand, SeekDirection, AudioBackendCommand};
use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable, RenderColor, RenderPanel, format_duration};
use pancurses::{Window, Input};
use std::fs::OpenOptions;
use std::io::Write;
use std::ffi::OsString;
use std::collections::HashMap;
use crossbeam::{unbounded, Sender, Receiver};
use std::thread;
use crate::playlists::{PlaylistManager, Song, Playlist};
use crate::config::{load_playlists, init_config, get_playlist_directory, Cache, FileManagerCache, PlaylistManagerCache};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use crate::play_state::{PlayPosition, PlayState, PlayMode};
use crate::debug_manager::DebugManager;

const FILE_BROWSER_OFFSET: i32 = 5;
const ESC_CHAR: char = 27 as char;
const ENTER_CHAR: char = 10 as char;
const CURSES_TIMEOUT: i32 = 200;
const QUEUE_OFFSET: Duration = Duration::from_secs(2);
const LOAD_OFFSET: Duration = Duration::from_secs(10);

pub struct Musicus {
    command_sender: Sender<AudioBackendCommand>,
	info_receiver: Receiver<AudioInfo>,
	file_manager: FileManager,
	playlist_manager: PlaylistManager,
	debug_manager: DebugManager,
	window: Window,
	color_pairs: HashMap<(RenderColor, RenderColor), i16>,
	color_pair_counter: i16,
	play_state: PlayState,
	view_state: ViewState,
	playing_song_info: Option<SongInfo>,
}

struct SongInfo {
	title: String,
	play_position: Duration,
	total_duration: Duration,
	queued_next: bool,
	loaded_next: bool,
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

		let cache = Cache::load();

		// setup curses
		let window = pancurses::initscr();
		Musicus::init_curses(&window);

		// setup audio backend
		let (audio_backend_sender, audio_backend_receiver) = unbounded();
        let (info_sender, info_receiver) = unbounded();

		let audio_backend_sender_clone = audio_backend_sender.clone();

		thread::spawn(move || {
			let mut audio_backend = AudioBackend::new(info_sender, audio_backend_sender_clone);
			audio_backend.run(audio_backend_receiver);
		});

		// load playlists
		let playlists = load_playlists();

		Musicus {
			command_sender: audio_backend_sender,
            info_receiver,
			file_manager: FileManager::new((window.get_max_y()-1) as usize, &cache.filemanager_cache),
			playlist_manager: PlaylistManager::new(playlists, &cache.playlist_manager_cache, (window.get_max_y()-1) as usize),
			debug_manager: DebugManager::new(),
			window,
			color_pairs: HashMap::new(),
			color_pair_counter: 1,
			play_state: PlayState::new(),
			view_state: cache.view,
			playing_song_info: None,
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
			filemanager_cache: FileManagerCache {
				current_directory: self.file_manager.current_path.clone(),
			},
			playlist_manager_cache: PlaylistManagerCache {
				view: self.playlist_manager.view,
			}
		};
		cache.dump();
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
		if let Some(song) = self.play_state.get_next_song(&self.playlist_manager) {
			self.command_sender.send(AudioBackendCommand::Command(AudioCommand::Play(song.clone()))).unwrap();
			self.play_state.next_song(&self.playlist_manager);
			if let PlayPosition::Playlist(playlist_index, song_index) = &mut self.play_state.play_position {
				if let Some(playlist) = &mut self.playlist_manager.playlists.get_mut(*playlist_index) {
					playlist.set_cursor_position(*song_index, self.playlist_manager.num_rows);
				}
			}
		} else {
			self.debug_manager.add_entry("no next song to play".to_string());
		}
	}

	fn handle_audio_backend(&mut self) -> bool {
		let mut has_to_render = false;
		for info in self.info_receiver.try_iter() {
			match info {
				AudioInfo::Playing(_song, play_position, total_duration) => {
					if let Some(playing_song) = &mut self.playing_song_info {
						playing_song.play_position = play_position;

						let duration_left = total_duration.checked_sub(play_position).unwrap_or(Duration::new(0, 0));

						// check for load command
						if !playing_song.loaded_next && duration_left < LOAD_OFFSET {
							if let Some(song) = self.play_state.get_next_song(&mut self.playlist_manager) {
								self.command_sender.send(AudioBackendCommand::Command(AudioCommand::Load(song.clone()))).unwrap();
								self.debug_manager.add_entry(format!("loading \"{}\"", song.title));
								playing_song.loaded_next = true;
							} else {
								self.debug_manager.add_entry("no next song to load".to_string());
							}
						}

						// check for queue command
						if !playing_song.queued_next && duration_left < QUEUE_OFFSET {
							if let Some(song) = self.play_state.get_next_song(&self.playlist_manager) {
								self.command_sender.send(AudioBackendCommand::Command(AudioCommand::Queue(song.clone()))).unwrap();
								self.play_state.next_song(&self.playlist_manager);
								self.debug_manager.add_entry(format!("queueing \"{}\"", song.title));
								playing_song.queued_next = true;
							} else {
								self.debug_manager.add_entry("no next song to queue".to_string());
							}
						}
					} else {
						self.debug_manager.add_entry_color("Got playing update, but playing song info is not set.".to_string(), RenderColor::RED, RenderColor::BLACK);
					}
				}
				AudioInfo::FailedOpen(song, e) => {
					self.debug_manager.add_entry_color(format!("Failed to open: {:?} {:?}\n", song.path, e), RenderColor::RED, RenderColor::BLACK);
				}
				AudioInfo::SongEnded(song) => {
					self.debug_manager.add_entry(format!("song ended: \"{}\"\n", song.title));
				}
				AudioInfo::SongStarts(song, total_duration, start_duration) => {
					self.playing_song_info = Some(SongInfo {
						title: song.title.clone(),
						play_position: start_duration,
						total_duration,
						queued_next: false,
						loaded_next: false
					});
					has_to_render = true;
					if start_duration == Duration::new(0, 0) {
						self.debug_manager.add_entry(format!("start song \"{}\"", song.title));
					}
				}
				_ => {}
			}
		}
		has_to_render
	}

	fn handle_input(&mut self, running: &mut bool) -> bool {
		let mut got_valid_input = false;
		if let Some(input) = self.window.getch() {
			match input {
				Input::Character(c) => {
					got_valid_input = true;
					match (c, self.view_state) {
						('q' | ESC_CHAR, _) => *running = false,
						('L', _) => self.seek(SeekDirection::Forward),
						('H', _) => self.seek(SeekDirection::Backward),
						('J', _) => self.start_next_song(),
						(ENTER_CHAR, ViewState::FileManager) => self.filemanager_context_action(),
						('y', ViewState::FileManager) => self.file_manager_add_to_playlist(),
						('n', ViewState::FileManager) => self.file_manager_new_playlist(),
						('h', ViewState::FileManager) => self.file_manager.move_left(),
						('j', ViewState::FileManager) => self.file_manager.move_down(),
						('k', ViewState::FileManager) => self.file_manager.move_up(),
						('l', ViewState::FileManager) => self.file_manager.move_right(),
						(ENTER_CHAR, ViewState::Playlists) => self.playlist_manager_context_action(),
						('h', ViewState::Playlists) => self.playlist_manager.move_left(),
						('l', ViewState::Playlists) => self.playlist_manager.move_right(),
						('j', ViewState::Playlists) => self.playlist_manager.move_down(),
						('k', ViewState::Playlists) => self.playlist_manager.move_up(),
						('c', _) => self.toggle_pause(),
						('1', _) => self.view_state = ViewState::FileManager,
						('2', _) => self.view_state = ViewState::Playlists,
						('3', _) => self.view_state = ViewState::Debug,
						('s', _) => self.play_state.toggle_mode(),
						('i', ViewState::FileManager) => self.playlist_manager.import_playlists(&self.file_manager.current_path),
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

	fn seek(&mut self, direction: SeekDirection) {
		let duration = Duration::from_secs(5);
		self.command_sender.send(
			AudioBackendCommand::Command(AudioCommand::Seek(SeekCommand {
				duration,
				direction,
			}))
		).unwrap();
		if let Some(playing_song) = &mut self.playing_song_info {
			playing_song.play_position = (playing_song.play_position + duration).min(playing_song.total_duration)
		}
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
		let current_path = self.file_manager.current_path.clone();
		let song = Song {
			title: current_path.file_name().unwrap_or(&OsString::from("<no filename>")).to_string_lossy().into_owned(),
			path: current_path,
		};
		Self::play(&self.command_sender, &mut self.play_state, song.clone());
		self.play_state.play_position = PlayPosition::File(song);
	}

	fn play(command_sender: &Sender<AudioBackendCommand>, play_state: &mut PlayState, song: Song) {
		command_sender.send(AudioBackendCommand::Command(AudioCommand::Play(song))).unwrap();
		play_state.playing = true;
	}

	fn playlist_manager_context_action(&mut self) {
        if let Some(song) = self.playlist_manager.get_shown_song() {
			Self::play(&self.command_sender, &mut self.play_state, song.clone());
			self.play_state.play_position = PlayPosition::Playlist(self.playlist_manager.shown_playlist_index, self.playlist_manager.get_shown_playlist().unwrap().cursor_position);
		}
	}

	fn file_manager_add_to_playlist(&mut self) {
		let songs = Song::songs_from_path(&self.file_manager.current_path);
		self.playlist_manager.add_songs(songs);
	}

	fn file_manager_new_playlist(&mut self) {
		let songs = Song::songs_from_path(&self.file_manager.current_path);
		let name = self.file_manager.current_path.file_name().unwrap().to_str().unwrap().replace(" ", "");

		let mut playlist = Playlist::new(name);
		playlist.songs = songs;
		self.playlist_manager.playlists.push(playlist);
	}

	fn render(&mut self, everything: bool) {
		if everything {
			let render_object = match self.view_state {
				ViewState::FileManager => self.file_manager.get_render_object(),
				ViewState::Playlists => self.playlist_manager.get_render_object(&self.play_state),
				ViewState::Debug => self.debug_manager.get_render_object(),
			};
			self.window.erase();
			self.render_panels(&render_object);
		}
		self.render_play_state();
		self.window.refresh();
	}

	fn render_panels(&mut self, render_object: &RenderObject) {
		let mut x_pos = (self.window.get_max_x() - (render_object.get_panels_size() as i32 + render_object.panels.len() as i32*FILE_BROWSER_OFFSET)).min(0);
		for panel in render_object.panels.iter() {
			self.render_panel(panel, x_pos);
			x_pos += panel.get_width() as i32 + FILE_BROWSER_OFFSET;
		}
	}

	fn render_play_state(&mut self) {
		self.set_color(RenderColor::BLACK, RenderColor::CYAN);
		if let Some(current_song) = &self.playing_song_info {
			self.window.mv(self.window.get_max_y() - 1, 0);
			self.window.hline(' ', self.window.get_max_x());
			let play_mode_str = match self.play_state.mode {
				PlayMode::Normal => " ",
				PlayMode::Shuffle => "S",
			};
			self.window.mvaddstr(
				self.window.get_max_y()-1,
				0,
				format!(
					" {} {}  {} / {}",
					play_mode_str,
					current_song.title,
					format_duration(current_song.play_position),
					format_duration(current_song.total_duration)
				),
			);
		}
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
				let line = &e.text[(-x_pos).max(0) as usize..];
				let line = format!("{: <width$}", line, width=panel.get_width() + FILE_BROWSER_OFFSET as usize);
				self.window.mvaddstr(y_pos as i32, x_pos.max(0), line);
			}
		}
	}
}

#[allow(unused)]
pub fn log(text: &str) {
	let mut file = OpenOptions::new().write(true).create(true).append(true).open("log.txt").expect("failed to open log file");
	file.write_all(text.as_bytes()).unwrap();
}
