use crate::render::{Renderable, RenderObject, RenderPanel, RenderEntry, RenderColor};

pub struct PlaylistManager {
	current_playlist: usize,
	playlists: Vec<Playlist>,
}

struct Playlist {
	name: String,
	songs: Vec<Song>,
}

struct Song {
	title: String,
}

impl PlaylistManager {
	pub fn new() -> PlaylistManager {
		let mut playlists = Vec::new();
		playlists.push(Playlist::new("my_playlist".to_string()));
		playlists.push(Playlist::new("my_playlist2".to_string()));
		PlaylistManager {
			current_playlist: 0,
			playlists,
		}
	}
}

impl Renderable for PlaylistManager {
	fn get_render_object(&self) -> RenderObject {
		let mut render_object = RenderObject::new();

		// add overview panel
		let mut overview_panel = RenderPanel::new(0);
		for playlist in &self.playlists {
			overview_panel.entries.push(RenderEntry::new(playlist.name.clone(), RenderColor::WHITE, RenderColor::BLACK));
		}
		render_object.panels.push(overview_panel);

		// add current playlist
		if let Some(current_playlist) = self.playlists.get(self.current_playlist) {
			let mut panel = RenderPanel::new(0);
			for song in &current_playlist.songs {
				panel.entries.push(RenderEntry::new(song.title.clone(), RenderColor::WHITE, RenderColor::BLACK));
			}
			render_object.panels.push(panel);
		}

		render_object
	}
}

impl Playlist {
	pub fn new(name: String) -> Playlist {
		Playlist {
			name,
			songs: Vec::new(),
		}
	}
}

impl Song {
	pub fn new(title: String) -> Song {
		Song {
			title,
		}
	}
}