use crate::file_manager::FileManager;

pub struct Musicus {
	file_manager: FileManager,
}

impl Musicus {
	pub fn new() -> Musicus {
		Musicus {
			file_manager: FileManager::new(),
		}
	}

	pub fn run(&mut self) {

	}
}