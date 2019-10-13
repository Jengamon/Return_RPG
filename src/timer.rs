use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    instant: Instant,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            instant: Instant::now()
        }
    }

    pub fn restart(&mut self) {
        self.instant = Instant::now();
    }

    pub fn frame(&mut self) -> Duration {
        let now = Instant::now();
        let dur = now - self.instant;
        self.instant = now;
        dur
    }
}