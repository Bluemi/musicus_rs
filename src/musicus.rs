use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable};
use pancurses::{Window, Input};

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
			file_manager: FileManager::new(),
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
					if c == 'q' {
						running = false;
					}
				}
				_ => {}
			}
		}
		pancurses::endwin();
	}

	fn render(&self, render_object: RenderObject) {
		self.render_panels(&render_object);
		self.window.refresh();
	}

	fn render_panels(&self, render_object: &RenderObject) {
		let mut x_pos = self.window.get_max_x();
		for panel in render_object.panels.iter().rev() {
			x_pos -= panel.get_width() as i32 + FILE_BROWSER_OFFSET;
			for (y_pos, e) in panel.entries.iter().enumerate() {
				self.window.mvaddstr(y_pos as i32, x_pos, &e.text);
			}
		}
	}
}