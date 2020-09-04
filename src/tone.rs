use rodio::Source;
use std::time::Duration;

/// Produces a sine for the specified time
///
/// Always has a rate of 48kHz and one channel.
#[derive(Clone, Debug)]
pub struct Tone {
    freq: f32,
    num_sample: usize,
    last_sample: usize,
    duration: Duration,
    sample_rate: u32,
}

impl Tone {
    /// The frequency of the sine.
    #[inline]
    pub fn new(freq: f32, duration: Duration) -> Tone {
        let sample_rate = 48000;

        let last_sample = duration.as_secs_f64() * sample_rate as f64;
        let last_sample = last_sample.round() as usize;

        Tone {
            freq: freq as f32,
            num_sample: 0,
            duration,
            last_sample,
            sample_rate,
        }
    }
}

impl Iterator for Tone {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        self.num_sample += 1;

        if self.num_sample > self.last_sample {
            return None;
        }

        let value = 2.0 * 3.14159265 * self.freq * self.num_sample as f32 / 48000.0;
        Some(value.sin())
    }
}

impl Source for Tone {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        1
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
