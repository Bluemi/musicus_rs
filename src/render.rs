use std::ffi::OsString;

pub struct RenderObject {
	pub panels: Vec<RenderPanel>,
}

pub struct RenderPanel {
	pub entries: Vec<RenderEntry>,
}

pub struct RenderEntry {
	pub text: String,
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
}

impl RenderPanel {
	pub fn new() -> RenderPanel {
		RenderPanel {
			entries: Vec::new(),
		}
	}
}

impl From<String> for RenderEntry {
	fn from(s: String) -> Self {
		RenderEntry {
			text: s,
		}
	}
}

impl From<OsString> for RenderEntry {
	fn from(s: OsString) -> Self {
		RenderEntry {
			text: s.into_string().unwrap(),
		}
	}
}
