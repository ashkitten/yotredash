use std::ops::Deref;
use time::Tm;

pub struct DerefInner<T>(pub T);

impl<T> Deref for DerefInner<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// A simple struct to count frames per second and update at a set interval
pub struct FpsCounter {
    /// The last time the counter reset
    last_time: Tm,
    /// The number of frames counted since the last reset
    frames: u32,
    /// Interval between resets
    interval: f32,
    /// Current frames per second
    fps: f32,
}

impl FpsCounter {
    pub fn new(interval: f32) -> Self {
        Self {
            last_time: ::time::now(),
            frames: 0,
            interval: interval,
            fps: 0.0,
        }
    }

    pub fn next_frame(&mut self) {
        self.frames += 1;
        let delta = ::time::now() - self.last_time;
        if delta > ::time::Duration::milliseconds((self.interval * 1_000.0) as i64) {
            self.fps = self.frames as f32 / (delta.num_nanoseconds().unwrap() as f32 / 1_000_000_000.0);
            self.frames = 0;
            self.last_time = ::time::now();
        }
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }
}
