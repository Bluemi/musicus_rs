use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable};
use pancurses::{Window, Input};
use std::fs::OpenOptions;
use std::io::Write;

const FILE_BROWSER_OFFSET: i32 = 5;

pub struct Musicus {
	file_manager: FileManager,
	window: Window,
}

impl Musicus {
	pub fn new() -> Musicus {
		let window = pancurses::initscr();
		pancurses::noecho();
		pancurses::curs_set(0);
		Musicus {
			file_manager: FileManager::new(window.get_max_y() as usize),
			window
		}
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
				if panel.cursor_position != y_pos+panel.scroll_position {
					self.window.mvaddstr(y_pos as i32, x_pos, &e.text);
				}
			}
		}
	}
}

pub fn log(text: &str) {
	let mut file = OpenOptions::new().write(true).create(true).append(true).open("log.txt").expect("failed to open log file");
	file.write_all(text.as_bytes()).unwrap();
}
