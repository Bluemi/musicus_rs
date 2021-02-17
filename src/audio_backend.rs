use std::path::Path;
use std::fs::File;
use std::io::BufReader;

pub struct AudioBackend {
	sink: rodio::Sink,
	stream: rodio::OutputStream,
	stream_handle: rodio::OutputStreamHandle,
}

impl AudioBackend {
	pub fn new() -> AudioBackend {
		let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		AudioBackend {
			sink: rodio::Sink::try_new(&stream_handle).unwrap(),
			stream,
			stream_handle,
		}
	}

	pub fn play(&mut self, file: &Path) {
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
}