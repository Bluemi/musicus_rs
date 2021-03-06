use rodio::{Source, Sample};
use std::time::Duration;
use crossbeam::Receiver;

pub struct Seekable<S> {
	source: S,
	seek_receiver: Receiver<Duration>
}

impl<S> Seekable<S>
where S: Source,
	  S::Item: Sample,
{
	pub fn new(source: S, seek_receiver: Receiver<Duration>) -> Seekable<S> {
		Seekable {
			source,
			seek_receiver,
		}
	}
}

impl<S> Iterator for Seekable<S>
where S: Source,
	  S::Item: Sample,
{
	type Item = S::Item;

	#[inline]
	fn next(&mut self) -> Option<S::Item> {
		let mut next = self.source.next();
		if let Ok(duration) = self.seek_receiver.try_recv() {
			let num_samples = (duration.as_secs_f64() * self.source.sample_rate() as f64) as usize;
			for _ in 0..num_samples {
				next = self.source.next();
			}
		}
		next
	}
}

impl<S> Source for Seekable<S>
where S: Source,
      S::Item: Sample,
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
