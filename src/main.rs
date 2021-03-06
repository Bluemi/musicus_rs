#![feature(or_patterns)]
#![feature(destructuring_assignment)]
#![feature(duration_consts_2)]

mod musicus;
mod file_manager;
mod render;
mod audio_backend;
mod playlists;
mod file_utils;
mod config;
mod done_access;
mod start_access;

use crate::musicus::Musicus;

fn main() {
	let mut musicus = Musicus::new();
	musicus.run();
}

