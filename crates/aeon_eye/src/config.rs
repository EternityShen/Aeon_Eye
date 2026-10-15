use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub global: GlobalConfig,
    pub targets: TargetsConfig,

    #[serde(default)]
    pub strategies: HashMap<String, AppStrategy>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobalConfig {
    #[serde(default = "default_scan_interval")]
    pub scan_interval: u64,

    #[serde(default = "default_decide_interval")]
    pub decide_interval: u64,

    #[serde(default = "default_shrike_cooldown")]
    pub shrink_cooldown: u64,

    #[serde(default = "default_false")]
    pub is_debug: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TargetsConfig {
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppStrategy {
    #[serde(default = "default_freeze_timeout")]
    pub freeze_timeout: u64,

    #[serde(default = "default_back_timeout")]
    pub back_timeout: u64,

    #[serde(default = "default_true")]
    pub enable_anon_shrink: bool,

    #[serde(default = "default_true")]
    pub enable_file_shrink: bool,

    #[serde(default)]
    pub aggressive: bool,
}

pub fn default_freeze_timeout() -> u64 {
    100
}

pub fn default_back_timeout() -> u64 {
    100
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_scan_interval() -> u64 {
    5
}
fn default_decide_interval() -> u64 {
    2
}

pub fn default_shrike_cooldown() -> u64 {
    100
}
