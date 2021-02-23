#![feature(or_patterns)]
#![feature(destructuring_assignment)]

mod musicus;
mod file_manager;
mod render;
mod audio_backend;
mod playlists;
mod file_utils;
mod config;

use crate::musicus::Musicus;

fn main() {
	let mut musicus = Musicus::new();
	musicus.run();
}
