use std::path::{PathBuf, Path};
use std::env::current_dir;
use crate::render::{Renderable, RenderObject, RenderPanel, RenderEntry};
use std::collections::HashMap;
use crate::musicus::log;

pub struct FileManager {
	pub current_path: PathBuf,
	pub positions: HashMap<PathBuf, (usize, usize)>, // maps Path to (cursor position, scroll position)
}

impl FileManager {
	pub fn new() -> FileManager {
		let current_path = current_dir().unwrap_or(PathBuf::new());
		FileManager {
			current_path,
			positions: HashMap::new(),
		}
	}

	pub fn move_left(&mut self) {
		self.current_path.pop();
	}

	pub fn move_right(&mut self) {
		let (cursor_position, _) = self.positions.get(&PathBuf::from(&self.current_path)).unwrap_or(&(0, 0));
		if let Ok(mut read_dir) = self.current_path.read_dir() {
			if let Some(Ok(dir_entry)) = read_dir.nth(*cursor_position) {
				if let Ok(ft) = dir_entry.file_type() {
					if ft.is_dir() {
						self.current_path = dir_entry.path();
					}
				}
			}
		}
		log(&format!("current path: {:?}", self.current_path))
	}

	pub fn move_up(&mut self) {
		let (cursor_position, scroll_position) = self.positions.entry(PathBuf::from(&self.current_path)).or_insert((0, 0));
		if *cursor_position > 0 {
			*cursor_position -= 1;
			if *scroll_position > *cursor_position {
				*scroll_position = *cursor_position;
			}
		}
		log(&format!("cp: {} sp: {}\n", *cursor_position, *scroll_position));
	}

	pub fn move_down(&mut self) {
		let (cursor_position, scroll_position) = self.positions.entry(PathBuf::from(&self.current_path)).or_insert((0, 0));
		*cursor_position += 1; // TODO: range check
		log(&format!("cp: {} sp: {}\n", *cursor_position, *scroll_position));
	}
}

impl Renderable for FileManager {
	fn get_render_object(&self) -> RenderObject {
		let mut render_object = RenderObject::new();
		for c in self.current_path.ancestors().collect::<Vec<&Path>>().iter().rev() {
			let (cursor_position, scroll_position) = self.positions.get(&PathBuf::from(c)).unwrap_or(&(0, 0));
			let mut panel = RenderPanel::new(*cursor_position, *scroll_position);
			for f in c.read_dir() {
				for i in f {
					panel.entries.push(RenderEntry::from(i.unwrap().file_name()));
				}
			}
			render_object.panels.push(panel);
		}

		render_object
	}
}