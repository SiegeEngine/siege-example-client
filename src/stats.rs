
pub struct Stats {
    pub network_clocksync_ms: i32,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            network_clocksync_ms: 999999,
        }
    }
}
