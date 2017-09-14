use time::Tm;

pub struct FpsCounter {
    last_time: Tm,
    frames: u32,
    interval: f32,
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
