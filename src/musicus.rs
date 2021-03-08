use crate::audio_backend::{AudioBackend, AudioCommand, AudioInfo, SeekCommand, SeekDirection};
use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable, RenderColor, RenderPanel, format_duration};
use pancurses::{Window, Input};
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashMap;
use crossbeam::{unbounded, Sender, Receiver};
use std::thread;
use crate::playlists::{PlaylistManager, Song, Playlist};
use crate::config::{load_playlists, init_config, get_playlist_directory, Cache, FileManagerCache, PlaylistManagerCache};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::Duration;

const FILE_BROWSER_OFFSET: i32 = 5;
const ESC_CHAR: char = 27 as char;
const ENTER_CHAR: char = 10 as char;
const CURSES_TIMEOUT: i32 = 200;

pub struct Musicus {
    command_sender: Sender<AudioCommand>,
	info_receiver: Receiver<AudioInfo>,
	file_manager: FileManager,
	playlist_manager: PlaylistManager,
	window: Window,
	color_pairs: HashMap<(RenderColor, RenderColor), i16>,
	color_pair_counter: i16,
	play_state: PlayState,
	view_state: ViewState,
	current_song_info: Option<CurrentSongInfo>,
}

struct CurrentSongInfo {
	title: String,
	current_duration: Duration,
	total_duration: Duration,
}

struct PlayState {
	playing: bool,
	play_position: PlayPosition,
}

enum PlayPosition {
	Empty,
	File(PathBuf),
	Playlist(usize, usize), // playlist index, song index
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum ViewState {
	FileManager,
	Playlists,
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
		let (command_sender, command_receiver) = unbounded();
        let (info_sender, info_receiver) = unbounded();


		thread::spawn(move || {
			let (update_sender, update_receiver) = unbounded();
			let mut audio_backend = AudioBackend::new(info_sender, update_sender);
			audio_backend.run(command_receiver, update_receiver);
		});

		// load playlists
		let playlists = load_playlists();

		Musicus {
            command_sender,
            info_receiver,
			file_manager: FileManager::new(window.get_max_y() as usize, &cache.filemanager_cache),
			playlist_manager: PlaylistManager::new(playlists, &cache.playlist_manager_cache, window.get_max_y() as usize),
			window,
			color_pairs: HashMap::new(),
			color_pair_counter: 1,
			play_state: PlayState::new(),
			view_state: cache.view,
			current_song_info: None,
		}
	}

	pub fn init_curses(window: &Window) {
		pancurses::noecho();
		pancurses::curs_set(0);
		pancurses::start_color();
		window.timeout(CURSES_TIMEOUT);
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
			self.handle_audio_backend();
			self.render(got_input);
		}
		self.shutdown();
	}

	fn handle_audio_backend(&mut self) {
		for info in self.info_receiver.try_iter() {
			match info {
				AudioInfo::Playing(_, duration) => {
					if let Some(current_song) = &mut self.current_song_info {
						current_song.current_duration = duration;
						log(&format!("playing update: {:?}\n", current_song.current_duration));
					}
				}
				AudioInfo::SongEndsSoon(_, _) => {
					match &mut self.play_state.play_position {
						PlayPosition::Playlist(playlist_index, song_index) => {
							*song_index += 1;
							if let Some(song) = self.playlist_manager.get_song(*playlist_index, *song_index) {
								self.command_sender.send(AudioCommand::Queue(song.path.clone())).unwrap();
							}
						}
						_ => {}
					}
				}
				AudioInfo::FailedOpen(path) => {
					log(&format!("Failed to open: {:?}\n", path));
				}
				AudioInfo::SongEnded(path) => {
					log(&format!("song ended: {:?}\n", path));
				}
				AudioInfo::SongStarts(_path, total_duration, start_duration) => {
					if let Some(current_song_info) = &mut self.current_song_info {
						current_song_info.total_duration = total_duration;
						current_song_info.current_duration = start_duration;
					}
					log(&format!("start update: {:?}\n", start_duration));
				}
				_ => {}
			}
		}
	}

	fn handle_input(&mut self, running: &mut bool) -> bool {
		let mut got_valid_input = false;
		if let Some(input) = self.window.getch() {
			match input {
				Input::Character(c) => {
					got_valid_input = true;
					match (c, self.view_state) {
						('q' | ESC_CHAR, _) => *running = false,
						('i', _) => self.seek(SeekDirection::Forward),
						('u', _) => self.seek(SeekDirection::Backward),
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
						('1', ViewState::Playlists) => self.view_state = ViewState::FileManager,
						('2', ViewState::FileManager) => self.view_state = ViewState::Playlists,
						_ => {
							got_valid_input = false;
							log(&format!("got unknown char: {}\n", c as i32));
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
		self.command_sender.send(AudioCommand::Seek(SeekCommand {
			duration,
			direction,
		})).unwrap();
		if let Some(current_song_info) = &mut self.current_song_info {
			current_song_info.current_duration = (current_song_info.current_duration + duration).min(current_song_info.total_duration)
		}
	}

	fn toggle_pause(&mut self) {
		if self.play_state.playing {
			self.command_sender.send(AudioCommand::Pause).unwrap();
		} else {
			self.command_sender.send(AudioCommand::Unpause).unwrap();
		}
		self.play_state.playing = !self.play_state.playing;
	}

	fn filemanager_context_action(&mut self) {
		let current_path = self.file_manager.current_path.clone();
		Self::play(&mut self.current_song_info, &self.command_sender, &mut self.play_state, current_path.clone(), None);
		self.play_state.play_position = PlayPosition::File(current_path);
	}

	fn play(current_song_info: &mut Option<CurrentSongInfo>, command_sender: &Sender<AudioCommand>, play_state: &mut PlayState, path: PathBuf, title: Option<String>) {
		let title = title.unwrap_or(path.to_string_lossy().into_owned());

		*current_song_info = Some(CurrentSongInfo {
			title,
			current_duration: Duration::new(0, 0),
			total_duration: Duration::new(0, 0),
		});

		command_sender.send(AudioCommand::Play(path)).unwrap();
		play_state.playing = true;
	}

	fn playlist_manager_context_action(&mut self) {
        if let Some(song) = self.playlist_manager.get_current_song() {
			Self::play(&mut self.current_song_info, &self.command_sender, &mut self.play_state, song.path.clone(), Some(song.title.clone()));
			self.play_state.play_position = PlayPosition::Playlist(self.playlist_manager.current_playlist, self.playlist_manager.get_current_playlist().unwrap().cursor_position);
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
				ViewState::Playlists => self.playlist_manager.get_render_object(),
			};
			self.window.clear();
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
		if let Some(current_song) = &self.current_song_info {
			self.window.mv(self.window.get_max_y() - 1, 0);
			self.window.hline(' ', self.window.get_max_x());
			self.window.mvaddstr(
				self.window.get_max_y()-1,
				0,
				format!(
					"{}  {} / {}",
					current_song.title,
					format_duration(current_song.current_duration),
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
		for (y_pos, e) in panel.entries.iter().skip(panel.scroll_position).take(self.window.get_max_y() as usize).enumerate() {
			self.set_color(e.foreground_color, e.background_color);
			if (e.text.len() as i32) >= -x_pos {
				let line = &e.text[(-x_pos).max(0) as usize..];
				let line = format!("{: <width$}", line, width=panel.get_width() + FILE_BROWSER_OFFSET as usize);
				self.window.mvaddstr(y_pos as i32, x_pos.max(0), line);
			}
		}
	}
}

impl PlayState {
	fn new() -> PlayState {
		PlayState {
			playing: false,
			play_position: PlayPosition::Empty,
		}
	}
}

#[allow(unused)]
pub fn log(text: &str) {
	let mut file = OpenOptions::new().write(true).create(true).append(true).open("log.txt").expect("failed to open log file");
	file.write_all(text.as_bytes()).unwrap();
}
