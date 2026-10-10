use std::time::{Duration, Instant};

use slot_cheevos::wifi::{Radio, Watcher};

#[derive(Default)]
struct Fake {
    link: bool,
    works: bool,
    ups: u32,
    downs: u32,
}

impl Radio for Fake {
    fn link_active(&self) -> bool {
        self.link
    }
    fn up(&mut self) -> bool {
        self.ups += 1;
        self.works
    }
    fn down(&mut self) {
        self.downs += 1;
    }
}

fn s(n: u64) -> Duration {
    Duration::from_secs(n)
}

#[test]
fn comes_up_straight_away_and_rechecks_later() {
    let t = Instant::now();
    let mut r = Fake {
        works: true,
        ..Default::default()
    };
    let mut w = Watcher::new(t);
    assert!(w.tick(&mut r, t));
    assert!(w.tick(&mut r, t + s(1)));
    assert_eq!(r.ups, 1);
    assert!(w.tick(&mut r, t + s(121)));
    assert_eq!(r.ups, 2);
}

#[test]
fn backs_off_after_failures() {
    let t = Instant::now();
    let mut r = Fake::default();
    let mut w = Watcher::new(t);
    assert!(!w.tick(&mut r, t));
    assert!(!w.tick(&mut r, t + s(29)));
    assert_eq!(r.ups, 1);
    w.tick(&mut r, t + s(30));
    assert_eq!(r.ups, 2);
    w.tick(&mut r, t + s(89));
    assert_eq!(r.ups, 2);
    w.tick(&mut r, t + s(90));
    assert_eq!(r.ups, 3);
    r.works = true;
    w.tick(&mut r, t + s(209));
    assert_eq!(r.ups, 3);
    assert!(w.tick(&mut r, t + s(210)));
}

#[test]
fn yields_the_radio_to_link_play_and_returns_after() {
    let t = Instant::now();
    let mut r = Fake {
        works: true,
        ..Default::default()
    };
    let mut w = Watcher::new(t);
    assert!(w.tick(&mut r, t));
    r.link = true;
    assert!(!w.tick(&mut r, t + s(1)));
    assert_eq!(r.downs, 1);
    assert!(!w.tick(&mut r, t + s(2)));
    assert_eq!((r.ups, r.downs), (1, 1));
    r.link = false;
    assert!(w.tick(&mut r, t + s(3)));
    assert_eq!(r.ups, 2);
}

#[test]
fn never_brings_wifi_up_during_link_play() {
    let t = Instant::now();
    let mut r = Fake {
        works: true,
        link: true,
        ..Default::default()
    };
    let mut w = Watcher::new(t);
    for i in 0..10 {
        assert!(!w.tick(&mut r, t + s(i * 100)));
    }
    assert_eq!((r.ups, r.downs), (0, 0));
}
