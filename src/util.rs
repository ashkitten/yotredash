//! Various utilities that don't really have a place elsewhere

use failure::Error;
use time::{self, Duration, Tm};

/// A simple struct to count frames per second and update at a set interval
pub struct FpsCounter {
    /// The last time the counter reset
    start: Tm,
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
            start: time::now(),
            frames: 0,
            interval: Duration::milliseconds((interval * 1_000.0) as i64),
            fps: 0.0,
        }
    }

    /// Increment the frame count and update the timer
    pub fn next_frame(&mut self) {
        self.frames += 1;
        let delta = time::now() - self.start;
        if delta > self.interval {
            let delta_nsec = delta.num_nanoseconds().unwrap() as f32 / 1_000_000_000.0;
            self.fps = self.frames as f32 / delta_nsec;
            self.frames = 0;
            self.start = time::now();
        }
    }

    /// Get the current FPS count
    pub fn fps(&self) -> f32 {
        self.fps
    }
}

pub fn format_error(error: &Error) -> String {
    let mut causes = error.iter_chain();
    format!(
        "{}{}",
        causes.next().unwrap(),
        causes
            .map(|cause| format!("\nCaused by: {}", cause))
            .collect::<Vec<String>>()
            .join("")
    )
}
