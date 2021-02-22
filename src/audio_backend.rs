use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::Receiver;

pub struct AudioBackend {
	sink: rodio::Sink,
	_stream: rodio::OutputStream,
	stream_handle: rodio::OutputStreamHandle,
	command_receiver: Receiver<AudioCommand>,
}

pub enum AudioCommand {
	Play(PathBuf),
	Pause,
    Unpause,
}

impl AudioBackend {
	pub fn new(command_receiver: Receiver<AudioCommand>) -> AudioBackend {
		let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		AudioBackend {
			sink: rodio::Sink::try_new(&stream_handle).unwrap(),
			_stream: stream,
			stream_handle,
			command_receiver,
		}
	}

	pub fn run(&mut self) {
		loop {
			match self.command_receiver.recv() {
				Ok(command) => self.handle_command(command),
				Err(e) => {
					println!("failed to receive audio command: {}. Shutting down AudioBackend", e);
					break;
				},
			}

		}
	}

	fn handle_command(&mut self, command: AudioCommand) {
		match command {
			AudioCommand::Play(path) => self.play(&path),
			AudioCommand::Pause => self.pause(),
			AudioCommand::Unpause => self.unpause(),
		}
	}

	fn play(&mut self, file: &Path) {
		if !self.sink.empty() {
			self.sink.stop();
			self.sink = rodio::Sink::try_new(&self.stream_handle).unwrap();
		}
		let file = File::open(file).unwrap();
		if let Ok(source) = rodio::Decoder::new(BufReader::new(file)) {
			self.sink.append(source);
		}
		self.sink.play();
	}

	fn pause(&mut self) {
		self.sink.pause();
	}

	fn unpause(&mut self) {
		self.sink.play();
	}
}