use rodio::{Source, Sample};
use std::time::Duration;

pub struct StartAccess<S, F> {
	source: S,
	modifier: F,
	signal_sent: bool,
}

impl<S, F> StartAccess<S, F>
where S: Source,
	  S::Item: Sample,
	  F: FnMut(),
{
	pub fn new(source: S, modifier: F) -> StartAccess<S, F> {
		StartAccess {
			source,
			modifier,
			signal_sent: false,
		}
	}
}

impl<S, F> Iterator for StartAccess<S, F>
where S: Source,
	  S::Item: Sample,
	  F: FnMut(),
{
	type Item = S::Item;

	#[inline]
	fn next(&mut self) -> Option<S::Item> {
		if !self.signal_sent {
			(self.modifier)();
			self.signal_sent = true;
		}
		self.source.next()
	}
}

impl<S, F> Source for StartAccess<S, F>
where S: Source,
      S::Item: Sample,
	  F: FnMut(),
{
	fn current_frame_len(&self) -> Option<usize> {
		self.source.current_frame_len()
	}

	fn channels(&self) -> u16 {
		self.source.channels()
	}

	fn sample_rate(&self) -> u32 {
		self.source.sample_rate()
	}

	fn total_duration(&self) -> Option<Duration> {
		self.source.total_duration()
	}
}

pub mod tests {
	#[test]
	pub fn test_start_access() {
		use rodio::Sink;

		let (sender, receiver) = crossbeam::unbounded();

		let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		let sink = rodio::Sink::try_new(&stream_handle).unwrap();

		// Add a dummy source of the sake of the example.
		let source = rodio::source::SineWave::new(220);
		let source = source.take_duration(Duration::new(2, 0));
		let source = StartAccess::new(source, move |_| { sender.send("hey"); });
		sink.append(source);
		let msg = receiver.recv().unwrap();
		sink.sleep_until_end();
		assert_eq!(msg, "hey");
	}
}
