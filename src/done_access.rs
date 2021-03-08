use rodio::{Source, Sample};
use std::time::Duration;

pub struct DoneAccess<S, F> {
	source: S,
	modifier: F,
	signal_sent: bool,
}

impl<S, F> DoneAccess<S, F>
where S: Source,
	  S::Item: Sample,
	  F: FnMut(&mut S),
{
	pub fn new(source: S, modifier: F) -> DoneAccess<S, F> {
		DoneAccess {
			source,
			modifier,
			signal_sent: false,
		}
	}
}

impl<S, F> Iterator for DoneAccess<S, F>
where S: Source,
	  S::Item: Sample,
	  F: FnMut(&mut S),
{
	type Item = S::Item;

	#[inline]
	fn next(&mut self) -> Option<S::Item> {
		let next = self.source.next();
		if !self.signal_sent && next.is_none() {
			(self.modifier)(&mut self.source);
			self.signal_sent = true;
		}
		next
	}
}

impl<S, F> Source for DoneAccess<S, F>
where S: Source,
      S::Item: Sample,
	  F: FnMut(&mut S),
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
	use super::*;

	#[test]
	fn test_done_access() {
		use rodio::Sink;

		let (sender, receiver) = crossbeam::unbounded();

		let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		let sink = rodio::Sink::try_new(&stream_handle).unwrap();
		sink.set_volume(0.0);

		// Add a dummy source of the sake of the example.
		let source = rodio::source::SineWave::new(440);
		let source = source.take_duration(Duration::new(2, 0));
		let source = DoneAccess::new(source, move |_| { sender.send("hey").unwrap(); } );
		sink.append(source);
		let msg = receiver.recv().unwrap();
		assert_eq!(msg, "hey");
	}
}
