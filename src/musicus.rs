use crate::file_manager::FileManager;
use crate::render::{RenderObject, Renderable, RenderColor, RenderPanel};
use pancurses::{Window, Input};
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashMap;

const FILE_BROWSER_OFFSET: i32 = 5;

pub struct Musicus {
	file_manager: FileManager,
	window: Window,
	color_pairs: HashMap<(RenderColor, RenderColor), i16>,
	color_pair_counter: i16,
}

impl Musicus {
	pub fn new() -> Musicus {
		let window = pancurses::initscr();
		Musicus::init_curses();
		Musicus {
			file_manager: FileManager::new(window.get_max_y() as usize),
			window,
			color_pairs: HashMap::new(),
			color_pair_counter: 1,
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
				let line = format!("{: <width$}", line, width=panel.get_width());
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
