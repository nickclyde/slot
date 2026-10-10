use std::time::{Duration, Instant};

const BACKOFF: [u64; 4] = [30, 60, 120, 300];
const RECHECK: Duration = Duration::from_secs(120);

pub trait Radio {
    fn link_active(&self) -> bool;
    fn up(&mut self) -> bool;
    fn down(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Off,
    Up,
    Yielded,
}

pub struct Watcher {
    state: State,
    next_try: Instant,
    failures: usize,
}

impl Watcher {
    pub fn new(now: Instant) -> Self {
        Self {
            state: State::Off,
            next_try: now,
            failures: 0,
        }
    }

    pub fn tick(&mut self, radio: &mut dyn Radio, now: Instant) -> bool {
        if radio.link_active() {
            if self.state == State::Up {
                radio.down();
            }
            self.state = State::Yielded;
            self.next_try = now;
            return false;
        }
        if now < self.next_try {
            return self.state == State::Up;
        }
        if radio.up() {
            self.state = State::Up;
            self.failures = 0;
            self.next_try = now + RECHECK;
            true
        } else {
            self.state = State::Off;
            self.next_try =
                now + Duration::from_secs(BACKOFF[self.failures.min(BACKOFF.len() - 1)]);
            self.failures += 1;
            false
        }
    }
}
