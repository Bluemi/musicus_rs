use crate::audio_backend::{AudioBackend, AudioCommand};
use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable, RenderColor, RenderPanel};
use pancurses::{Window, Input};
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use crate::playlists::PlaylistManager;

const FILE_BROWSER_OFFSET: i32 = 5;
const ESC_CHAR: char = 27 as char;
const ENTER_CHAR: char = 10 as char;

pub struct Musicus {
    command_sender: Sender<AudioCommand>,
	file_manager: FileManager,
	playlists: PlaylistManager,
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

#[derive(Copy, Clone)]
enum ViewState {
	FileManager,
	Playlists,
}

impl Musicus {
	pub fn new() -> Musicus {
        // setup curses
		let window = pancurses::initscr();
		Musicus::init_curses();

		// setup audio backend
		let (command_sender, command_receiver) = channel();

		thread::spawn(move || {
			let mut audio_backend = AudioBackend::new(command_receiver);
			audio_backend.run();
		});
		Musicus {
            command_sender,
			file_manager: FileManager::new(window.get_max_y() as usize),
			playlists: PlaylistManager::new(),
			window,
			color_pairs: HashMap::new(),
			color_pair_counter: 1,
			play_state: PlayState::Paused,
			view_state: ViewState::FileManager,
		}
	}

	pub fn init_curses() {
		pancurses::noecho();
		pancurses::curs_set(0);
		pancurses::start_color();
	}

	pub fn run(&mut self) {
		let mut running = true;
		while running {
			let render_object = match self.view_state {
				ViewState::FileManager => self.file_manager.get_render_object(),
				ViewState::Playlists => self.playlists.get_render_object(),
			};
			self.render(render_object);
			match self.window.getch().unwrap() {
				Input::Character(c) => {
					match (c, self.view_state) {
						('q' | ESC_CHAR, _) => running = false,
						(ENTER_CHAR, ViewState::FileManager) => self.play_filemanager_song(),
						('h', ViewState::FileManager) => self.file_manager.move_left(),
						('j', ViewState::FileManager) => self.file_manager.move_down(),
						('k', ViewState::FileManager) => self.file_manager.move_up(),
						('l', ViewState::FileManager) => self.file_manager.move_right(),
						('c', _) => self.toggle_pause(),
						('1', ViewState::Playlists) => self.view_state = ViewState::FileManager,
						('2', ViewState::FileManager) => self.view_state = ViewState::Playlists,
						_ => log(&format!("got unknown char: {}", c as i32)),
					}
				}
				_ => {}
			}
		}
		pancurses::endwin();
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

	fn play_filemanager_song(&mut self) {
		self.command_sender.send(AudioCommand::Play(self.file_manager.current_path.clone())).unwrap();
		self.play_state = PlayState::Playing;
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
