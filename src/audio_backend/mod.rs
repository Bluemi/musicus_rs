use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossbeam::{Receiver, Select, Sender};
use rodio::{Sink, Source};

use done_access::DoneAccess;
use periodic_access::PeriodicAccess;
use start_access::StartAccess;

use crate::musicus::log;

mod done_access;
mod start_access;
mod periodic_access;

const UPDATE_DURATION: Duration = Duration::from_millis(100);
const SONG_ENDS_SOON_OFFSET: Duration = Duration::from_millis(2000);

pub struct AudioBackend {
	sink: rodio::Sink,
	_stream: rodio::OutputStream,
	stream_handle: rodio::OutputStreamHandle,
    info_sender: Sender<AudioInfo>, // sender for info to musicus
	update_sender: Sender<AudioUpdate>, // internal updates for source state
	current_song: Option<CurrentSongState>, //
}

struct CurrentSongState {
	path: PathBuf,
	total_duration: Duration,
	current_duration: Duration,
	start_duration: Duration,
	sent_song_ends_soon: bool,
}

impl CurrentSongState {
	fn get_real_current_duration(&self) -> Duration {
		self.current_duration + self.start_duration
	}

	fn set_real_current_duration(&mut self, duration: Duration) {
		self.current_duration = duration.checked_sub(self.start_duration).unwrap_or(Duration::new(0, 0));
	}
}

pub enum AudioCommand {
	Play(PathBuf),
	Queue(PathBuf),
	Pause,
    Unpause,
	Seek(SeekCommand),
}

pub struct SeekCommand {
	pub duration: Duration,
	pub direction: SeekDirection,
}

pub enum SeekDirection {
	Forward,
	Backward,
}

#[derive(Debug)]
pub enum AudioInfo {
	Playing(PathBuf, Duration), // current song, current duration
	Queued(PathBuf),
	SongStarts(PathBuf, Duration, Duration),
	SongEndsSoon(PathBuf, Duration),
	FailedOpen(PathBuf),
	SongEnded(PathBuf),
}

#[derive(Debug)]
pub enum AudioUpdate {
	Playing(PathBuf, Duration), // current song, left duration
	SongStarts(PathBuf, Duration, Duration), // current song, total duration, start duration
	SongEnded(PathBuf),
}

impl AudioBackend {
	pub fn new(info_sender: Sender<AudioInfo>, update_sender: Sender<AudioUpdate>) -> AudioBackend {
		let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		AudioBackend {
			sink: rodio::Sink::try_new(&stream_handle).unwrap(),
			_stream: stream,
			stream_handle,
			info_sender,
			update_sender,
			current_song: None,
		}
	}

	pub fn run(&mut self, command_receiver: Receiver<AudioCommand>, update_receiver: Receiver<AudioUpdate>) {
		let mut select = Select::new();
		let command_index = select.recv(&command_receiver);
		let update_index = select.recv(&update_receiver);

		loop {
			let oper = select.select();
			match oper.index() {
				i if i == command_index => {
					let command = oper.recv(&command_receiver);
					match command {
						Ok(command) => self.handle_command(command),
						Err(_) => break,
					}
				},
				i if i == update_index => {
					let update = oper.recv(&update_receiver);
					match update {
						Ok(update) => self.handle_update(update),
						Err(_) => break,
					}
				}
				_ => unreachable!(),
			}
		}
	}

	fn handle_command(&mut self, command: AudioCommand) {
		match command {
			AudioCommand::Play(path) => Self::play(&mut self.sink, &self.stream_handle, &self.update_sender, &self.info_sender, &path, None),
			AudioCommand::Queue(path) => Self::queue(&mut self.sink, &self.update_sender, &self.info_sender, &path, None),
			AudioCommand::Pause => self.pause(),
			AudioCommand::Unpause => self.unpause(),
			AudioCommand::Seek(seek_command) => self.seek(seek_command),
		}
	}

	fn handle_update(&mut self, update: AudioUpdate) {
		match update {
			AudioUpdate::Playing(path, duration_left) => {
				if let Some(current_song) = &mut self.current_song {
					assert_eq!(current_song.path, path);
					if !current_song.sent_song_ends_soon && duration_left <= SONG_ENDS_SOON_OFFSET {
						self.info_sender.send(AudioInfo::SongEndsSoon(path.clone(), duration_left)).unwrap();
						current_song.sent_song_ends_soon = true;
					}
					current_song.set_real_current_duration(current_song.total_duration - duration_left);
					// log(&format!("playing update: {:?}\n", current_song.get_real_current_duration()));
					self.info_sender.send(AudioInfo::Playing(path.clone(), current_song.get_real_current_duration())).unwrap();
				} else {
					log(&format!("ERROR: current song is None, but got Playing update\n"));
				}
			}
			AudioUpdate::SongEnded(path) => {
				self.info_sender.send(AudioInfo::SongEnded(path)).unwrap();
				self.current_song = None;
			}
			AudioUpdate::SongStarts(path, total_duration, start_duration) => {
				self.info_sender.send(AudioInfo::SongStarts(path.clone(), total_duration, start_duration)).unwrap();
				self.current_song = Some(CurrentSongState {
					path: path.to_path_buf(),
					total_duration,
					current_duration: Duration::new(0, 0),
					start_duration,
					sent_song_ends_soon: false,
				});
				// log(&format!("start update: {:?}\n", start_duration));
			}
		}
	}

	fn seek(&mut self, seek_command: SeekCommand) {
		if let Some(current_song) = &mut self.current_song {
			current_song.start_duration = match seek_command.direction {
				SeekDirection::Forward => {
					(current_song.get_real_current_duration() + seek_command.duration).min(current_song.total_duration)
				}
				SeekDirection::Backward => {
					current_song.get_real_current_duration().checked_sub(seek_command.duration).unwrap_or(Duration::new(0, 0))
				}
			};
			current_song.current_duration = Duration::new(0, 0);
			// log(&format!("seek: {:?}\n", current_song.get_real_current_duration()));
			Self::play(&mut self.sink, &self.stream_handle, &self.update_sender, &self.info_sender, &current_song.path, Some(current_song.get_real_current_duration()));
		}
	}

	fn play(
		sink: &mut Sink,
		stream_handle: &rodio::OutputStreamHandle,
		update_sender: &Sender<AudioUpdate>,
		info_sender: &Sender<AudioInfo>,
		path: &Path,
		skip: Option<Duration>,
	) {
		if !sink.empty() {
			sink.stop();
			*sink = rodio::Sink::try_new(stream_handle).unwrap();
		}
		Self::queue(sink, update_sender, info_sender, &path, skip);
		sink.play();
	}

	fn queue(
		sink: &mut Sink,
		orig_update_sender: &Sender<AudioUpdate>,
		info_sender: &Sender<AudioInfo>,
		path: &Path,
		skip: Option<Duration>,
	) {
		match File::open(path) {
			Ok(file) => {
				if let Ok(decoder) = rodio::Decoder::new(BufReader::new(file)) {
					if let Some(total_duration) = decoder.total_duration() {
						// add start info
						let update_sender = orig_update_sender.clone();
						let path_buf = path.to_path_buf();
						let start_access_source = StartAccess::new(
							decoder,
							move || update_sender.send(AudioUpdate::SongStarts(path_buf.clone(), total_duration, skip.unwrap_or(Duration::new(0, 0)))).unwrap(),
						);

						// add playing info
						let update_sender = orig_update_sender.clone();
						let path_buf = path.to_path_buf();
						let periodic_access_source = PeriodicAccess::new(
							start_access_source,
							move |s| update_sender.send(AudioUpdate::Playing(path_buf.clone(), s.total_duration().unwrap_or(Duration::new(0, 0)))).unwrap(),
							UPDATE_DURATION
						);

						// add done info
						let update_sender = orig_update_sender.clone();
						let path_buf = path.to_path_buf();
						let done_access_source = DoneAccess::new(
							periodic_access_source,
							move |_| update_sender.send(AudioUpdate::SongEnded(path_buf.clone())).unwrap(),
						);

						if let Some(duration) = skip {
							let source = done_access_source.skip_duration(duration);
							sink.append(source);
						} else {
							sink.append(done_access_source);
						}

						info_sender.send(AudioInfo::Queued(path.to_path_buf())).unwrap();
					} else {
						info_sender.send(AudioInfo::FailedOpen(path.to_path_buf())).unwrap();
					}
				} else {
					info_sender.send(AudioInfo::FailedOpen(path.to_path_buf())).unwrap();
				}
			}
			Err(_) => {}
		}
	}

	fn pause(&mut self) {
		self.sink.pause();
	}

	fn unpause(&mut self) {
		self.sink.play();
	}
}
