// Copyright 2014 Rich Lane.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
