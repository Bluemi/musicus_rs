use std::path::{PathBuf, Path};
use std::env::current_dir;
use crate::render::{Renderable, RenderObject, RenderPanel, RenderEntry};

pub struct FileManager {
	pub current_path: PathBuf,
}

impl FileManager {
	pub fn new() -> FileManager {
		let current_path = current_dir().unwrap_or(PathBuf::new());
		FileManager {
			current_path,
		}
	}

	pub fn test(&self) {
	}
}

impl Renderable for FileManager {
	fn get_render_object(&self) -> RenderObject {
		let mut render_object = RenderObject::new();
		for c in self.current_path.ancestors().collect::<Vec<&Path>>().iter().rev() {
			let mut panel = RenderPanel::new();
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