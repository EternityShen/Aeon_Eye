use crate::config::Config;
use crate::platform::{cgroup, system};
use crate::share::app_snapshot::AppSnapshot;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

pub struct ShareData {
    pub uid_map: HashMap<String, u32>,
    pub config: Arc<Config>,
    pub in_freezer_app_map: HashMap<String, AppSnapshot>,
    pub in_backend_app_map: HashMap<String, AppSnapshot>,
    pub in_dead_app_map: HashMap<String, AppSnapshot>,
    pub in_top_app: Option<String>,
    pub in_suppress_app_set: HashSet<String>,
    pub shrunk_cooldown: Instant,
    pub cpu_cores: usize,
    pub zram_usage_percent: f64,
    pub is_screen: bool,
}

impl ShareData {
    pub fn new(config: Arc<Config>) -> Self {
        let mut uid_map = HashMap::new();
        let in_freezer_app_map = HashMap::new();
        let in_backend_app_map = HashMap::new();
        let in_dead_app_map = HashMap::new();
        let in_suppress_app_set = HashSet::new();

        let cpu_cores = system::get_cpu_cores();
        let zram_usage_percent = system::get_zram_usage_percent();

        for package in &config.targets.packages {
            let result = cgroup::get_uid_by_package(package);

            match result {
                Ok(uid) => {
                    println!("{} -> {}", package, uid);
                    uid_map.insert(package.clone(), uid);
                }
                Err(e) => {
                    println!("无法获取Uid: {}", e);
                }
            }
        }

        Self {
            uid_map,
            config,

            in_freezer_app_map,
            in_backend_app_map,
            in_dead_app_map,
            in_top_app: None,
            in_suppress_app_set,
            shrunk_cooldown: Instant::now(),

            cpu_cores,
            zram_usage_percent,
            is_screen: true,
        }
    }

    pub fn get_uid_map(&self) -> &HashMap<String, u32> {
        &self.uid_map
    }

    pub fn get_config(&self) -> &Arc<Config> {
        &self.config
    }

    pub fn get_in_freezer_app_map(&self) -> &HashMap<String, AppSnapshot> {
        &self.in_freezer_app_map
    }

    pub fn get_in_backend_app_map(&self) -> &HashMap<String, AppSnapshot> {
        &self.in_backend_app_map
    }

    pub fn get_in_dead_app_map(&self) -> &HashMap<String, AppSnapshot> {
        &self.in_dead_app_map
    }

    pub fn get_in_top_app(&self) -> &Option<String> {
        &self.in_top_app
    }

    pub fn get_cpu_cores(&self) -> usize {
        self.cpu_cores
    }

    pub fn get_zram_usage_percent(&self) -> f64 {
        self.zram_usage_percent
    }
}
