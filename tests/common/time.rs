use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

fn clock() -> &'static Mutex<SystemTime> {
    static CLOCK: OnceLock<Mutex<SystemTime>> = OnceLock::new();
    CLOCK.get_or_init(|| Mutex::new(SystemTime::now()))
}

pub fn set_now(time: SystemTime) {
    let mut guard = clock().lock().expect("clock poisoned");
    *guard = time;
}

pub fn advance(duration: Duration) {
    let mut guard = clock().lock().expect("clock poisoned");
    *guard = guard
        .checked_add(duration)
        .expect("duration advance overflowed");
}

pub fn now() -> SystemTime {
    *clock().lock().expect("clock poisoned")
}
