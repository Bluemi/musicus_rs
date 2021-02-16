use std::path::{PathBuf, Path};
use std::env::current_dir;
use crate::render::{Renderable, RenderObject, RenderPanel, RenderEntry, RenderColor};
use std::collections::HashMap;
use std::fs::DirEntry;

pub struct FileManager {
	pub current_path: PathBuf,
	pub positions: HashMap<PathBuf, (usize, usize)>, // maps Path to (cursor position, scroll position)
	pub num_rows: usize,
}

impl FileManager {
	pub fn new(num_rows: usize) -> FileManager {
		let current_path = current_dir().unwrap_or(PathBuf::new());

		let mut positions = HashMap::new();

		for (dir, root) in current_path.ancestors().zip(current_path.ancestors().skip(1)) {
			for (i, entry) in FileManager::get_dir_entries(root).iter().enumerate() {
				if entry.path == dir {
					positions.insert(PathBuf::from(root), (i, 0));
				}
			}
		}
		FileManager {
			current_path,
			positions,
			num_rows,
		}
	}

	pub fn move_left(&mut self) {
		self.current_path.pop();
	}

	pub fn move_right(&mut self) {
		let (cursor_position, _) = self.positions.get(&PathBuf::from(&self.current_path)).unwrap_or(&(0, 0));

		if let Some(dir_entry) = FileManager::get_dir_entries(&self.current_path).iter().nth(*cursor_position) {
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

	fn move_cursor_down(&mut self) {
		let num_entries = self.get_current_num_entries();
		let (cursor_position, scroll_position) = self.positions.entry(PathBuf::from(&self.current_path)).or_insert((0, 0));
		if *cursor_position < num_entries-1 {
			*cursor_position += 1;
			*scroll_position = (*scroll_position as i32).max(*cursor_position as i32-self.num_rows as i32 + 1) as usize;
		}
	}

	pub fn move_down(&mut self) {
		self.move_left();
		self.move_cursor_down();
		self.move_right();
	}

	fn get_current_num_entries(&self) -> usize {
		FileManager::get_dir_entries(&self.current_path).len()
	}

	fn get_dir_entries(path: &Path) -> Vec<DirectoryEntry> {
		let mut entries = Vec::new();
		if let Ok(read_dir) = path.read_dir() {
			for entry in read_dir {
				if let Ok(entry) = entry {
					let entry = DirectoryEntry::from(entry);
					if !entry.filename.starts_with(".") {
						entries.push(entry);
					}
				}
			}
		}
		entries.sort();
		entries
	}
}

impl Renderable for FileManager {
	fn get_render_object(&self) -> RenderObject {
		let mut render_object = RenderObject::new();
		for c in self.current_path.ancestors().collect::<Vec<&Path>>().iter().rev() {
			let (cursor_position, scroll_position) = self.positions.get(&PathBuf::from(c)).unwrap_or(&(0, 0));
			let mut panel = RenderPanel::new(*cursor_position, *scroll_position);
			for entry in FileManager::get_dir_entries(c) {
				let color = if entry.is_file { RenderColor::WHITE } else { RenderColor::BLUE };
				panel.entries.push(RenderEntry::new(entry.filename, color));
			}
			render_object.panels.push(panel);
		}

		render_object
	}
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct DirectoryEntry {
	pub is_file: bool,
	pub filename: String,
	pub path: PathBuf,
}

impl From<DirEntry> for DirectoryEntry {
	fn from(dir_entry: DirEntry) -> Self {
		DirectoryEntry {
			filename: dir_entry.file_name().into_string().unwrap(),
			is_file: dir_entry.file_type().map_or(true, |de| de.is_file()),
			path: dir_entry.path(),
		}
	}
}

