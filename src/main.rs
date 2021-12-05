#![feature(destructuring_assignment)]
#![feature(duration_consts_2)]

use crate::musicus::Musicus;

mod musicus;
mod file_manager;
mod render;
mod audio_backend;
mod playlist_manager;
mod config;
mod play_state;
mod debug_manager;
mod song;
mod string_helpers;

fn main() {
	let mut musicus = Musicus::new();
	musicus.run();
}

