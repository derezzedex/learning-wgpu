use std::time::{Duration, Instant};

pub struct Timer{
    accumulator: Duration,
    delta: Duration,
    current: Instant,
}

impl Timer{
    pub fn new() -> Self{
        let accumulator = Duration::new(0, 0);
        let delta = Duration::new(0, 0);
        let current = Instant::now();

        Self{
            accumulator,
            delta,
            current,
        }
    }

    pub fn reset(&mut self){
        let now = Instant::now();
        self.delta = now - self.current;
        self.current = now;
        self.accumulator += self.delta;
    }

    pub const UPS: u64 = 20;
    pub fn should_update(&self) -> bool{
        self.accumulator >= Duration::from_millis(1000 / Self::UPS)
    }

    pub fn update(&mut self){
        self.accumulator -= Duration::from_millis(1000 / Self::UPS);
    }

    pub fn get_delta(&self) -> Duration{
        self.delta
    }
}
