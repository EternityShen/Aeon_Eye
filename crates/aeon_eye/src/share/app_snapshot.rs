use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub enum AppStatus {
    Back { since: Instant },
    Freezer { since: Instant },
    Top,
    Dead,
}

#[derive(Clone, Debug)]
pub struct AppSnapshot {
    pub uid: u32,
    pub adj: i32,
    pub pids: Vec<i32>,
    pub status: AppStatus,
}
