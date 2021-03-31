use std::collections::HashMap;
use std::mem::swap;
use std::path::{Path, PathBuf};

use crate::config::FileManagerCache;
use crate::render::{Renderable, RenderColor, RenderEntry, RenderObject, RenderPanel, Alignment};
use crate::file_manager::file_utils::{normalize_dir, get_dir_entries};

pub mod file_utils;

pub struct FileManager {
	pub current_path: PathBuf,
	pub positions: HashMap<PathBuf, (usize, usize)>, // maps Path to (cursor position, scroll position)
}

impl FileManager {
	pub fn new(cache: &FileManagerCache) -> FileManager {
		let mut current_path = cache.current_directory.clone();

		normalize_dir(&mut current_path);

		let mut positions = HashMap::new();

		for (dir, root) in current_path.ancestors().zip(current_path.ancestors().skip(1)) {
			for (i, entry) in get_dir_entries(root).iter().enumerate() {
				if entry.path == dir {
					positions.insert(PathBuf::from(root), (i, 0));
				}
			}
		}
		FileManager {
			current_path,
			positions,
		}
	}

	pub fn move_left(&mut self) {
		self.current_path.pop();
	}

	pub fn move_right(&mut self) {
		let (cursor_position, _) = self.positions.get(&PathBuf::from(&self.current_path)).unwrap_or(&(0, 0));

		if let Some(dir_entry) = get_dir_entries(&self.current_path).iter().nth(*cursor_position) {
			self.current_path = dir_entry.path.clone();
		}
	}

	fn move_cursor_up(&mut self) {
		let (cursor_position, scroll_position) = self.positions.entry(PathBuf::from(&self.current_path)).or_insert((0, 0));
		if *cursor_position > 0 {
			*cursor_position -= 1;
			if *scroll_position > *cursor_position {
				*scroll_position = *cursor_position;
			}
		}
	}

	pub fn move_up(&mut self) {
		self.move_left();
		self.move_cursor_up();
		self.move_right();
	}

	fn move_cursor_down(&mut self, num_rows: usize) {
		let num_entries = self.get_current_num_entries();
		let (cursor_position, scroll_position) = self.positions.entry(PathBuf::from(&self.current_path)).or_insert((0, 0));
		if *cursor_position < num_entries-1 {
			*cursor_position += 1;
			*scroll_position = (*scroll_position as i32).max(*cursor_position as i32-num_rows as i32 + 1) as usize;
		}
	}

	pub fn move_down(&mut self, num_rows: usize) {
		self.move_left();
		self.move_cursor_down(num_rows);
		self.move_right();
	}

	fn get_current_num_entries(&self) -> usize {
		get_dir_entries(&self.current_path).len()
	}
}

impl Renderable for FileManager {
	fn get_render_object(&self) -> RenderObject {
		let mut render_object = RenderObject::new(Alignment::Right);
		let ancestors = self.current_path.ancestors().collect::<Vec<&Path>>();
		for (ancestor_index, ancestor) in ancestors.iter().rev().enumerate() {
			let (cursor_position, scroll_position) = self.positions.get(&PathBuf::from(ancestor)).unwrap_or(&(0, 0));
			let mut panel = RenderPanel::new(*scroll_position);
			let dir_entries = get_dir_entries(ancestor);
			for (entry_index, entry) in dir_entries.iter().enumerate() {
				let mut foreground_color = if entry.is_file {
					RenderColor::WHITE
				} else {
					RenderColor::BLUE
				};
				let mut background_color = RenderColor::BLACK;
				if entry_index == *cursor_position && ancestor_index != ancestors.len()-1 {
					swap(&mut foreground_color, &mut background_color);
				}
				panel.entries.push(RenderEntry::new(entry.filename.clone(), foreground_color, background_color));
			}
			render_object.panels.push(panel);
		}

		render_object
	}
}
