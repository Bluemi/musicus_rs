use crate::render::{RenderObject, RenderPanel, RenderEntry, RenderColor};

pub struct DebugManager {
    entries: Vec<Entry>,
    has_update: bool,
    scroll_position: usize,
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
            has_update: false,
            scroll_position: 0,
        }
    }

    pub fn scroll(&mut self, direction: i32) {
        self.scroll_position = (self.scroll_position as i32 + direction).max(0) as usize;
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
        self.has_update = true;
    }

    pub fn get_render_object(&self) -> RenderObject {
        let mut render_object = RenderObject::new();
        let mut render_panel = RenderPanel::new(0);
        for entry in &self.entries {
            let render_entry = RenderEntry::new(entry.text.clone(), entry.foreground_color, entry.background_color);
            render_panel.entries.push(render_entry);
        }
        if self.entries.is_empty() {
            render_panel.entries.push(RenderEntry::new("<no entries>".to_string(), RenderColor::BLUE, RenderColor::BLACK));
        }
		render_panel.scroll_position = self.scroll_position;
        render_object.panels.push(render_panel);

        render_object
    }

    pub fn has_update(&mut self) -> bool {
        let hu = self.has_update;
        self.has_update = false;
        hu
    }
}