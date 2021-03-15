use crate::render::{RenderObject, RenderPanel, RenderEntry, RenderColor};

pub struct DebugManager {
    entries: Vec<String>,
}

impl DebugManager {
    pub fn new() -> DebugManager {
        DebugManager {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: String) {
        self.entries.push(entry);
    }

    pub fn get_render_object(&self) -> RenderObject {
        let mut render_object = RenderObject::new();
        let mut render_panel = RenderPanel::new(0);
        for entry in &self.entries {
            let render_entry = RenderEntry::new(entry.clone(), RenderColor::WHITE, RenderColor::BLACK);
            render_panel.entries.push(render_entry);
        }
        render_object.panels.push(render_panel);

        render_object
    }
}