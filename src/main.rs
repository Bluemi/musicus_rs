#![feature(or_patterns)]
#![feature(destructuring_assignment)]
#![feature(duration_consts_2)]

use crate::musicus::Musicus;

mod musicus;
mod file_manager;
mod render;
mod audio_backend;
mod playlists;
mod config;

fn main() {
	let mut musicus = Musicus::new();
	musicus.run();
}

