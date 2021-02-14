use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable, RenderColor};
use pancurses::{Window, Input, COLOR_WHITE, COLOR_BLACK, COLOR_BLUE};
use std::fs::OpenOptions;
use std::io::Write;

const FILE_BROWSER_OFFSET: i32 = 5;

const NORMAL_COLOR: i16 = 0;
const SELECTED_COLOR: i16 = 1;
const DIRECTORY_COLOR: i16 = 2;
const SELECTED_DIRECTORY_COLOR: i16 = 3;

pub struct Musicus {
	file_manager: FileManager,
	window: Window,
}

impl Musicus {
	pub fn new() -> Musicus {
		let window = pancurses::initscr();
		Musicus::init_curses();
		Musicus {
			file_manager: FileManager::new(window.get_max_y() as usize),
			window
		}
	}

	pub fn init_curses() {
		pancurses::noecho();
		pancurses::curs_set(0);
		pancurses::start_color();
		pancurses::init_pair(NORMAL_COLOR, COLOR_WHITE, COLOR_BLACK);
		pancurses::init_pair(SELECTED_COLOR, COLOR_BLACK, COLOR_WHITE);
		pancurses::init_pair(DIRECTORY_COLOR, COLOR_BLUE, COLOR_BLACK);
		pancurses::init_pair(SELECTED_DIRECTORY_COLOR, COLOR_BLACK, COLOR_BLUE);
	}

	pub fn run(&mut self) {
		let mut running = true;
		while running {
			let render_object = self.file_manager.get_render_object();
			self.render(render_object);
			match self.window.getch().unwrap() {
				Input::Character(c) => {
					match c {
						'q' => running = false,
						'h' => self.file_manager.move_left(),
						'j' => self.file_manager.move_down(),
						'k' => self.file_manager.move_up(),
						'l' => self.file_manager.move_right(),
						_ => {},
					}
				}
				_ => {}
			}
		}
		pancurses::endwin();
	}

	fn render(&self, render_object: RenderObject) {
		self.window.clear();
		self.render_panels(&render_object);
		self.window.refresh();
	}

	fn render_panels(&self, render_object: &RenderObject) {
		let mut x_pos = self.window.get_max_x();
		for panel in render_object.panels.iter().rev() {
			x_pos -= panel.get_width() as i32 + FILE_BROWSER_OFFSET;
			for (y_pos, e) in panel.entries.iter().skip(panel.scroll_position).enumerate() {
				if panel.cursor_position == y_pos+panel.scroll_position {
					match e.color {
						RenderColor::WHITE => self.window.color_set(SELECTED_COLOR),
						RenderColor::BLUE => self.window.color_set(SELECTED_DIRECTORY_COLOR),
					};
				} else {
					match e.color {
						RenderColor::WHITE => self.window.color_set(NORMAL_COLOR),
						RenderColor::BLUE => self.window.color_set(DIRECTORY_COLOR),
					};
				}
				self.window.mvaddstr(y_pos as i32, x_pos, &e.text);
			}
		}
	}
}

#[allow(unused)]
pub fn log(text: &str) {
	let mut file = OpenOptions::new().write(true).create(true).append(true).open("log.txt").expect("failed to open log file");
	file.write_all(text.as_bytes()).unwrap();
}
