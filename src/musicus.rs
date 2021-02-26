use crate::audio_backend::{AudioBackend, AudioCommand, AudioInfo};
use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable, RenderColor, RenderPanel};
use pancurses::{Window, Input};
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashMap;
use crossbeam::{unbounded, Sender, Receiver, TryRecvError};
use std::thread;
use crate::playlists::{PlaylistManager, Song, Playlist};
use crate::config::{load_playlists, init_config, get_playlist_directory, Cache, FileManagerCache, PlaylistManagerCache};
use serde::{Serialize, Deserialize};

const FILE_BROWSER_OFFSET: i32 = 5;
const ESC_CHAR: char = 27 as char;
const ENTER_CHAR: char = 10 as char;

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
}

enum PlayState {
	Playing,
	Paused,
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
		Musicus::init_curses();

		// setup audio backend
		let (command_sender, command_receiver) = unbounded();
        let (info_sender, info_receiver) = unbounded();


		thread::spawn(move || {
			let mut audio_backend = AudioBackend::new(command_receiver, info_sender);
			audio_backend.run();
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
			play_state: PlayState::Paused,
			view_state: cache.view,
		}
	}

	pub fn init_curses() {
		pancurses::noecho();
		pancurses::curs_set(0);
		pancurses::start_color();
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
		while running {
			let render_object = match self.view_state {
				ViewState::FileManager => self.file_manager.get_render_object(),
				ViewState::Playlists => self.playlist_manager.get_render_object(),
			};
			self.render(render_object);
			match self.window.getch().unwrap() {
				Input::Character(c) => {
					match (c, self.view_state) {
						('q' | ESC_CHAR, _) => running = false,
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
						_ => log(&format!("got unknown char: {}", c as i32)),
					}
				}
				_ => {}
			}
			loop {
				match self.info_receiver.try_recv() {
					Ok(info) => {
						// TODO
						match info {
							AudioInfo::Playing(_, _) => {}
							AudioInfo::DurationLeft(_, _) => {}
							AudioInfo::FailedOpen(_) => {}
						}
					}
					Err(e) => {
                        match e {
							TryRecvError::Empty => {
								break;
							}
							TryRecvError::Disconnected => {
								log(&format!("failed to recv info! {:?}", e));
							}
						}
					}
				}
			}
		}
		self.shutdown();
	}

	fn toggle_pause(&mut self) {
		match self.play_state {
			PlayState::Playing => {
				self.command_sender.send(AudioCommand::Pause).unwrap();
				self.play_state = PlayState::Paused;
			}
			PlayState::Paused => {
				self.command_sender.send(AudioCommand::Unpause).unwrap();
				self.play_state = PlayState::Playing;
			}
		}
	}

	fn filemanager_context_action(&mut self) {
		self.command_sender.send(AudioCommand::Play(self.file_manager.current_path.clone())).unwrap();
		self.play_state = PlayState::Playing;
	}

	fn playlist_manager_context_action(&mut self) {
        if let Some(song) = self.playlist_manager.get_current_song() {
			self.command_sender.send(AudioCommand::Play(song.path.clone())).unwrap();
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

	fn render(&mut self, render_object: RenderObject) {
		self.window.clear();
		self.render_panels(&render_object);
		self.window.refresh();
	}

	fn render_panels(&mut self, render_object: &RenderObject) {
		let mut x_pos = (self.window.get_max_x() - (render_object.get_panels_size() as i32 + render_object.panels.len() as i32*FILE_BROWSER_OFFSET)).min(0);
		for panel in render_object.panels.iter() {
			self.render_panel(panel, x_pos);
			x_pos += panel.get_width() as i32 + FILE_BROWSER_OFFSET;
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

#[allow(unused)]
pub fn log(text: &str) {
	let mut file = OpenOptions::new().write(true).create(true).append(true).open("log.txt").expect("failed to open log file");
	file.write_all(text.as_bytes()).unwrap();
}
