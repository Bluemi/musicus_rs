pub struct RenderObject {
	pub panels: Vec<RenderPanel>,
}

pub struct RenderPanel {
	pub entries: Vec<RenderEntry>,
	pub cursor_position: usize,
	pub scroll_position: usize,
}

pub struct RenderEntry {
	pub text: String,
	pub color: RenderColor,
}

pub enum RenderColor {
	WHITE,
	BLUE
}

pub trait Renderable {
	fn get_render_object(&self) -> RenderObject;
}

impl RenderObject {
	pub fn new() -> RenderObject {
		RenderObject {
			panels: Vec::new(),
		}
	}

	pub fn get_panels_size(&self) -> usize {
		self.panels.iter().map(|p| p.get_width()).sum()
	}
}

impl RenderPanel {
	pub fn new(cursor_position: usize, scroll_position: usize) -> RenderPanel {
		RenderPanel {
			entries: Vec::new(),
			cursor_position,
			scroll_position
		}
	}

	pub fn get_width(&self) -> usize {
		let mut width = 0;
		for e in &self.entries {
			width = width.max(e.text.len());
		}
		width
	}
}

impl RenderEntry {
	pub fn new(text: String, color: RenderColor) -> Self {
		RenderEntry {
			text,
			color,
		}
	}
}
