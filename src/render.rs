use pancurses::{COLOR_WHITE, COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_YELLOW, COLOR_RED};
use std::time::Duration;

pub struct RenderObject {
	pub panels: Vec<RenderPanel>,
	pub alignment: Alignment,
}

pub enum Alignment {
	Left,
	Right,
}

pub struct RenderPanel {
	pub entries: Vec<RenderEntry>,
	pub scroll_position: usize,
}

pub struct RenderEntry {
	pub text: String,
	pub foreground_color: RenderColor,
	pub background_color: RenderColor,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum RenderColor {
	Black,
	White,
	Blue,
	Cyan,
	Yellow,
	Red,
}

pub trait Renderable {
	fn get_render_object(&self) -> RenderObject;
}

impl RenderObject {
	pub fn new(alignment: Alignment) -> RenderObject {
		RenderObject {
			panels: Vec::new(),
			alignment,
		}
	}

	pub fn get_panels_size(&self) -> usize {
		self.panels.iter().map(|p| p.get_width()).sum()
	}
}

impl RenderPanel {
	pub fn new(scroll_position: usize) -> RenderPanel {
		RenderPanel {
			entries: Vec::new(),
			scroll_position
		}
	}

	pub fn get_width(&self) -> usize {
		let mut width = 0;
		for e in &self.entries {
			width = width.max(e.get_width());
		}
		width
	}
}

impl RenderEntry {
	pub fn new(text: String, foreground_color: RenderColor, background_color: RenderColor) -> Self {
		RenderEntry {
			text,
			foreground_color,
			background_color,
		}
	}

	pub fn get_width(&self) -> usize {
		self.text.len()
	}
}

impl RenderColor {
	pub fn to_curses_color(self) -> i16 {
		match self {
			RenderColor::Black => COLOR_BLACK,
			RenderColor::White => COLOR_WHITE,
			RenderColor::Blue => COLOR_BLUE,
			RenderColor::Cyan => COLOR_CYAN,
			RenderColor::Yellow => COLOR_YELLOW,
			RenderColor::Red => COLOR_RED,
		}
	}
}

pub fn format_duration(duration: Duration) -> String {
	let total_seconds = duration.as_secs();
	let seconds = total_seconds % 60;
	let minutes = (total_seconds / 60) % 60;
	let hours = total_seconds / 3600;
	if hours > 0 {
		format!("{}:{:0.2}:{:02}", hours, minutes, seconds)
	} else {
		format!("{:0.2}:{:02}", minutes, seconds)
	}
}