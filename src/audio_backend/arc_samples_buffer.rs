use std::sync::Arc;
use std::time::Duration;
use rodio::Source;

pub struct ArcSamplesBuffer {
    pub sound: Arc<Sound>,
    index: usize,
}

impl ArcSamplesBuffer {
    pub fn new(sound: Arc<Sound>) -> ArcSamplesBuffer {
        ArcSamplesBuffer {
            sound,
            index: 0,
        }
    }
}

impl Iterator for ArcSamplesBuffer {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        let sample = self.sound.data.get(self.index);
        self.index += 1;
        sample.cloned()
    }
}

impl Source for ArcSamplesBuffer {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.sound.channels
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.sound.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        Some(self.sound.duration)
    }
}

pub struct Sound {
    pub channels: u16,
    pub sample_rate: u32,
    pub duration: Duration,
    pub data: Vec<f32>,
    pub counter: usize,
}

