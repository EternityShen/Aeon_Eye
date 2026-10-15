pub enum Action {
    OnShrinkAnon { pkg: String, pids: Vec<i32> },
    OffShrink { pkg: String },
    OnShrinkFile { pkg: String, pids: Vec<i32> },
}
