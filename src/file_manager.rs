use std::path::{PathBuf, Path};
use std::env::current_dir;
use crate::render::{Renderable, RenderObject, RenderPanel, RenderEntry};
use std::collections::HashMap;
use crate::musicus::log;

pub struct FileManager {
	pub current_path: PathBuf,
	pub positions: HashMap<PathBuf, (usize, usize)>, // maps Path to (cursor position, scroll position)
	pub num_rows: usize,
}

impl FileManager {
	pub fn new(num_rows: usize) -> FileManager {
		let current_path = current_dir().unwrap_or(PathBuf::new());
		FileManager {
			current_path,
			positions: HashMap::new(),
			num_rows,
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
	}

	pub fn move_up(&mut self) {
		let (cursor_position, scroll_position) = self.positions.entry(PathBuf::from(&self.current_path)).or_insert((0, 0));
		if *cursor_position > 0 {
			*cursor_position -= 1;
			if *scroll_position > *cursor_position {
				*scroll_position = *cursor_position;
			}
		}
	}

	pub fn move_down(&mut self) {
		let num_entries = self.get_current_num_entries();
		let (cursor_position, scroll_position) = self.positions.entry(PathBuf::from(&self.current_path)).or_insert((0, 0));
		if *cursor_position < num_entries-1 {
			*cursor_position += 1; // TODO: range check
			*scroll_position = (*scroll_position as i32).max(*cursor_position as i32-self.num_rows as i32 + 1) as usize;
		}
	}

	fn get_current_num_entries(&self) -> usize {
		self.current_path.read_dir().unwrap().count()
	}
}

impl Renderable for FileManager {
	fn get_render_object(&self) -> RenderObject {
		let mut render_object = RenderObject::new();
		for c in self.current_path.ancestors().collect::<Vec<&Path>>().iter().rev() {
			let (cursor_position, scroll_position) = self.positions.get(&PathBuf::from(c)).unwrap_or(&(0, 0));
			let mut panel = RenderPanel::new(*cursor_position, *scroll_position);
			if let Ok(read_dir) = c.read_dir() {
				for i in read_dir {
					panel.entries.push(RenderEntry::from(i.unwrap().file_name()));
				}
			}
			render_object.panels.push(panel);
		}

		render_object
	}
}