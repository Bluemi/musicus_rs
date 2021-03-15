use crate::render::{RenderObject, RenderPanel, RenderEntry, RenderColor};

pub struct DebugManager {
    entries: Vec<Entry>,
}

struct Entry {
    pub text: String,
    pub foreground_color: RenderColor,
    pub background_color: RenderColor,
}

impl DebugManager {
    pub fn new() -> DebugManager {
        DebugManager {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, text: String) {
        self.add_entry_color(text, RenderColor::WHITE, RenderColor::BLACK);
    }

    pub fn add_entry_color(&mut self, text: String, foreground_color: RenderColor, background_color: RenderColor) {
        self.entries.push(Entry {
            text,
            foreground_color,
            background_color
        });
    }

    pub fn get_render_object(&self) -> RenderObject {
        let mut render_object = RenderObject::new();
        let mut render_panel = RenderPanel::new(0);
        for entry in &self.entries {
            let render_entry = RenderEntry::new(entry.text.clone(), entry.foreground_color, entry.background_color);
            render_panel.entries.push(render_entry);
        }
        render_object.panels.push(render_panel);

        render_object
    }
}