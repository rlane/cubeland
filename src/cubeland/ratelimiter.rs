extern mod extra;

use extra::time::precise_time_ns;

pub struct RateLimiter {
    next_time: u64,
    interval: u64,
}

impl RateLimiter {
    pub fn new(interval: u64) -> RateLimiter {
        RateLimiter{next_time: precise_time_ns(), interval: interval}
    }

    pub fn limit(&mut self) -> bool {
        let now = precise_time_ns();
        if now >= self.next_time {
            self.next_time = now + self.interval;
            true
        } else {
            false
        }
    }
}
