use std::path::PathBuf;
use std::env::current_dir;

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
}