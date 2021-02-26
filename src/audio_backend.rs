use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufReader;
use rodio::Source;
use std::time::Duration;
use crossbeam::{Sender, Receiver};
use crate::musicus::log;

const UPDATE_DURATION: Duration = Duration::new(1, 0);

pub struct AudioBackend {
	sink: rodio::Sink,
	_stream: rodio::OutputStream,
	stream_handle: rodio::OutputStreamHandle,
	command_receiver: Receiver<AudioCommand>,
    info_sender: Sender<AudioInfo>,
}

pub enum AudioCommand {
	Play(PathBuf),
	Queue(PathBuf),
	Pause,
    Unpause,
}

pub enum AudioInfo {
	Playing(PathBuf, Duration), // current song, left duration
	DurationLeft(PathBuf, Duration),
	FailedOpen(PathBuf),
}

impl AudioBackend {
	pub fn new(command_receiver: Receiver<AudioCommand>, info_sender: Sender<AudioInfo>) -> AudioBackend {
		let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		AudioBackend {
			sink: rodio::Sink::try_new(&stream_handle).unwrap(),
			_stream: stream,
			stream_handle,
			command_receiver,
			info_sender,
		}
	}

	pub fn run(&mut self) {
		loop {
			match self.command_receiver.recv() {
				Ok(command) => self.handle_command(command),
				Err(_) => break,
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

	fn play(&mut self, path: &Path) {
		if !self.sink.empty() {
			self.sink.stop();
			self.sink = rodio::Sink::try_new(&self.stream_handle).unwrap();
		}
		self.queue(path);
		self.sink.play();
	}

	fn queue(&mut self, path: &Path) {
		log(&format!("queue"));
		match File::open(path) {
			Ok(file) => {
				if let Ok(source) = rodio::Decoder::new(BufReader::new(file)) {
					let info_sender = self.info_sender.clone();
					let path_buf = path.to_path_buf();
					self.info_sender.send(AudioInfo::Playing(path.to_path_buf(), source.total_duration().unwrap())).unwrap();
					let source = source.periodic_access(
						UPDATE_DURATION,
						move |s| info_sender.send(AudioInfo::DurationLeft(path_buf.clone(), s.total_duration().unwrap_or(Duration::new(0, 0)))).unwrap()
					);
					self.sink.append(source);
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