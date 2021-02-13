use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable};
use pancurses::{Window, Input};

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
			let i = self.window.getch().unwrap();
			match i {
				Input::Character(c) => {
					if c == 'q' {
						running = false;
					}
				}
				Input::KeyDown => {}
				Input::KeyUp => {}
				Input::KeyLeft => {}
				Input::KeyRight => {}
				_ => {}
			}
		}
		pancurses::endwin();
	}

	fn render(&self, render_object: RenderObject) {
		self.window.mvaddstr(10, 10, "heyhey");
		self.window.refresh();
	}
}