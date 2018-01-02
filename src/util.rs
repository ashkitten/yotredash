//! Various utilities that don't really have a place elsewhere

use std::ops::Deref;
use time::Duration;

/// A struct that derefs to its contents
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
    counter: Duration,
    /// The number of frames counted since the last reset
    frames: u32,
    /// Interval between resets
    interval: Duration,
    /// Current frames per second
    fps: f32,
}

impl FpsCounter {
    /// Create a new instance with the specified update interval
    pub fn new(interval: f32) -> Self {
        Self {
            counter: Duration::zero(),
            frames: 0,
            interval: Duration::milliseconds((interval * 1_000.0) as i64),
            fps: 0.0,
        }
    }

    /// Increment the frame count and update the timer
    pub fn next_frame(&mut self, delta: Duration) {
        self.frames += 1;
        self.counter = self.counter + delta;
        if self.counter > self.interval {
            self.fps = self.frames as f32 / (self.counter.num_nanoseconds().unwrap() as f32 / 1_000_000_000.0);
            self.frames = 0;
            self.counter = self.counter - self.interval
        }
    }

    /// Get the current FPS count
    pub fn fps(&self) -> f32 {
        self.fps
    }
}
