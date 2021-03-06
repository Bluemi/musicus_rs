use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufReader;
use rodio::Source;
use std::time::Duration;
use crossbeam::{unbounded, Sender, Receiver, select};
use crate::done_access::DoneAccess;
use crate::start_access::StartAccess;

const UPDATE_DURATION: Duration = Duration::from_millis(200);

pub struct AudioBackend {
	sink: rodio::Sink,
	_stream: rodio::OutputStream,
	stream_handle: rodio::OutputStreamHandle,
	command_receiver: Receiver<AudioCommand>, // commands from musicus
    info_sender: Sender<AudioInfo>, // sender for info to musicus
	update_receiver: Receiver<AudioUpdate>, // internal updates for source state
	update_sender: Sender<AudioUpdate>, // internal updates for source state
}

pub enum AudioCommand {
	Play(PathBuf),
	Queue(PathBuf),
	Pause,
    Unpause,
}

#[derive(Debug)]
pub enum AudioInfo {
	Playing(PathBuf, Duration), // current song, left duration
	Queued(PathBuf),
	SongStarts(PathBuf),
	SongEndsSoon(PathBuf, Duration),
	FailedOpen(PathBuf),
	SongEnded(PathBuf),
}

#[derive(Debug)]
enum AudioUpdate {
	Playing(PathBuf, Duration), // current song, left duration
	SongStarts(PathBuf),
	SongEnded(PathBuf),
}

impl AudioBackend {
	pub fn new(command_receiver: Receiver<AudioCommand>, info_sender: Sender<AudioInfo>) -> AudioBackend {
		let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		let (update_sender, update_receiver) = unbounded();
		AudioBackend {
			sink: rodio::Sink::try_new(&stream_handle).unwrap(),
			_stream: stream,
			stream_handle,
			command_receiver,
			info_sender,
			update_receiver,
			update_sender,
		}
	}

	pub fn run(&mut self) {
		loop {
			select! {
				recv(self.command_receiver) -> command => {
					match command {
						Ok(command) => self.handle_command(command),
						Err(_) => break,
					}
				},
				recv(self.update_receiver) -> update => {
					match update {
						Ok(update) => self.handle_update(update),
						Err(_) => break,
					}
				}
			}
		}
	}

	fn handle_command(&mut self, command: AudioCommand) {
		match command {
			AudioCommand::Play(path) => self.play(&path),
			AudioCommand::Queue(path) => self.queue(&path),
			AudioCommand::Pause => self.pause(),
			AudioCommand::Unpause => self.unpause(),
		}
	}

	fn handle_update(&mut self, update: AudioUpdate) {
		match update {
			AudioUpdate::Playing(path, duration) => {
				self.info_sender.send(AudioInfo::Playing(path.clone(), duration)).unwrap();
				if duration <= UPDATE_DURATION {
					self.info_sender.send(AudioInfo::SongEndsSoon(path.clone(), duration)).unwrap();
				}
			},
			AudioUpdate::SongEnded(path) => {
				self.info_sender.send(AudioInfo::SongEnded(path)).unwrap();
			}
			AudioUpdate::SongStarts(path) => {
				self.info_sender.send(AudioInfo::SongStarts(path)).unwrap();
			}
		}
	}

	fn play(&mut self, path: &Path) {
		if !self.sink.empty() {
			self.sink.stop();
			self.sink = rodio::Sink::try_new(&self.stream_handle).unwrap();
		}
		self.queue(path);
		self.sink.play();
	}

	fn queue(&mut self, path: &Path) {
		match File::open(path) {
			Ok(file) => {
				if let Ok(source) = rodio::Decoder::new(BufReader::new(file)) {
					if let Some(_total_duration) = source.total_duration() {
						// add start info
						let update_sender = self.update_sender.clone();
						let path_buf = path.to_path_buf();
						let source = StartAccess::new(
							source,
							move || update_sender.send(AudioUpdate::SongStarts(path_buf.clone())).unwrap(),

						);

						// add playing info
						let update_sender = self.update_sender.clone();
						let path_buf = path.to_path_buf();
						let source = source.periodic_access(
							UPDATE_DURATION,
							move |s| update_sender.send(AudioUpdate::Playing(path_buf.clone(), s.total_duration().unwrap_or(Duration::new(0, 0)))).unwrap()
						);

						// add done info
						let update_sender = self.update_sender.clone();
						let path_buf = path.to_path_buf();
						let source = DoneAccess::new(
							source,
							move |_| update_sender.send(AudioUpdate::SongEnded(path_buf.clone())).unwrap(),
						);

						self.sink.append(source);

						self.info_sender.send(AudioInfo::Queued(path.to_path_buf())).unwrap();
					} else {
						self.info_sender.send(AudioInfo::FailedOpen(path.to_path_buf())).unwrap();
					}
				} else {
					self.info_sender.send(AudioInfo::FailedOpen(path.to_path_buf())).unwrap();
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