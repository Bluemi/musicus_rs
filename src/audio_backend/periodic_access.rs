use rodio::{Source, Sample};
use std::time::Duration;
use crate::musicus::log;

pub struct PeriodicAccess<S, F> {
	source: S,
	modifier: F,
	update_frequency: u32,
	samples_until_update: u32,
	duration_played: Duration,
	period: Duration,
}

impl<S, F> PeriodicAccess<S, F>
where S: Source,
	  S::Item: Sample,
	  F: FnMut(&mut S, Duration),
{
	pub fn new(source: S, modifier: F, period: Duration) -> PeriodicAccess<S, F> {
		let update_frequency = (period.as_secs_f64() * source.sample_rate() as f64) as u32;

		PeriodicAccess {
			source,
			modifier,
			update_frequency,
			samples_until_update: 1,
			duration_played: Duration::new(0, 0),
			period,
		}
	}
}

impl<S, F> Iterator for PeriodicAccess<S, F>
where S: Source,
	  S::Item: Sample,
	  F: FnMut(&mut S, Duration),
{
	type Item = S::Item;

	#[inline]
	fn next(&mut self) -> Option<S::Item> {
		let next = self.source.next();

		self.samples_until_update -= 1;
		if self.samples_until_update == 0 {
			(self.modifier)(&mut self.source, self.duration_played);
			self.duration_played += self.period;
			self.samples_until_update = self.update_frequency;
		}

		next
	}
}

impl<S, F> Source for PeriodicAccess<S, F>
where S: Source,
      S::Item: Sample,
	  F: FnMut(&mut S, Duration),
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
	#[allow(unused_imports)]
	use super::*;

	#[test]
	fn test_periodic_access() {
		use rodio::Sink;

		let (sender, receiver) = crossbeam::unbounded();

		let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		let sink = rodio::Sink::try_new(&stream_handle).unwrap();
		sink.set_volume(0.0);

		// Add a dummy source of the sake of the example.
		let source = rodio::source::SineWave::new(440);
		let source = source.take_duration(Duration::new(2, 0));
		let source = PeriodicAccess::new(source, move |_| { sender.send("hey").unwrap(); }, Duration::new(1, 0) );
		sink.append(source);
		let msg = receiver.recv().unwrap();
		assert_eq!(msg, "hey");
	}
}
