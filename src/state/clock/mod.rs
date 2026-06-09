#[derive(Clone)]
pub struct Clock;

impl Clock {
    pub fn new() -> Self {
        Clock
    }

    pub fn now_s(&self) -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_secs() as i64
    }

    pub fn now_ms(&self) -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_millis() as i64
    }
}
