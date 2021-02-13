mod musicus;
mod file_manager;
mod render;

use crate::musicus::Musicus;

fn main() {
	let mut musicus = Musicus::new();
	musicus.run();
}
