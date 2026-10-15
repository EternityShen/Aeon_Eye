use crate::platform::{cgroup, proc, system};
use crate::share::app_snapshot::{AppSnapshot, AppStatus};
use crate::share::data::ShareData;
use logger::Logger;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

const TOP_ADJ: i32 = 400;

pub struct SharedUpdater {
    shared_data_lock: Arc<RwLock<ShareData>>,
    logger: Arc<Mutex<Logger>>,
}

impl SharedUpdater {
    pub fn new(shared_data_lock: Arc<RwLock<ShareData>>, logger: Arc<Mutex<Logger>>) -> Self {
        Self {
            shared_data_lock,
            logger,
        }
    }

    pub fn scan(&self) -> HashMap<String, AppSnapshot> {
        let mut scan_map = HashMap::new();
        let top_package = system::get_top_package();

        let (scan_interval, packages, uid_map, screen) = match self.shared_data_lock.read() {
            Ok(shared_data) => (
                shared_data.config.global.scan_interval,
                shared_data.config.targets.packages.clone(),
                shared_data.uid_map.clone(),
                shared_data.is_screen,
            ),
            Err(_) => return scan_map,
        };

        if !screen {
            return scan_map;
        }

        if scan_interval > 0 {
            std::thread::sleep(Duration::from_secs(scan_interval));
        }

        let pids_map = proc::find_pids_for_packages(&packages);
        for package in &packages {
            let Some(uid) = uid_map.get(package).copied() else {
                continue;
            };
            let pids = pids_map.get(package);
            let is_top_package = top_package
                .as_deref()
                .is_some_and(|pkg| package.trim() == pkg.trim());
            let adj = pids
                .and_then(|v| v.first())
                .and_then(|pid| proc::get_adj(*pid))
                .unwrap_or(800);

            let status = if pids.map_or(true, |v| v.is_empty()) {
                AppStatus::Dead
            } else if is_top_package {
                AppStatus::Top
            } else if cgroup::is_frozen_uid(uid) {
                AppStatus::Freezer {
                    since: Instant::now(),
                }
            } else if adj >= TOP_ADJ {
                AppStatus::Back {
                    since: Instant::now(),
                }
            } else {
                AppStatus::Top
            };

            scan_map.insert(
                package.trim().to_string(),
                AppSnapshot {
                    uid,
                    pids: pids.cloned().unwrap_or_default(),
                    status,
                    adj,
                },
            );
        }
        scan_map
    }

    pub fn category(&mut self, scan_map: HashMap<String, AppSnapshot>) {
        if let Ok(mut shared_data) = self.shared_data_lock.write() {
            if !shared_data.is_screen {
                return;
            }
            for (pkg, snapshot) in &scan_map {
                match snapshot.status {
                    AppStatus::Freezer { .. } => {
                        if shared_data.in_top_app.as_deref() == Some(pkg.as_str()) {
                            shared_data.in_top_app = None;
                        }

                        if let Some(existing) = shared_data.in_freezer_app_map.get_mut(pkg) {
                            existing.adj = snapshot.adj;
                            existing.pids = snapshot.pids.clone();
                        } else {
                            shared_data
                                .in_freezer_app_map
                                .insert(pkg.clone(), snapshot.clone());

                            if shared_data.config.global.is_debug {
                                if let Ok(mut log) = self.logger.lock() {
                                    log.debug(format!("进入Freezer:{}=>{:?}", pkg, snapshot));
                                }
                            }

                            shared_data.in_backend_app_map.remove(pkg);
                            shared_data.in_dead_app_map.remove(pkg);
                        }
                    }
                    AppStatus::Top => {
                        if shared_data.in_top_app.as_deref() != Some(pkg.as_str()) {
                            shared_data.in_top_app = Some(pkg.clone());
                        }

                        shared_data.in_backend_app_map.remove(pkg);
                        shared_data.in_dead_app_map.remove(pkg);
                        shared_data.in_freezer_app_map.remove(pkg);
                    }
                    AppStatus::Back { .. } => {
                        if shared_data.in_top_app.as_deref() == Some(pkg.as_str()) {
                            shared_data.in_top_app = None;
                        }

                        if let Some(existing) = shared_data.in_backend_app_map.get_mut(pkg) {
                            existing.adj = snapshot.adj;
                            existing.pids = snapshot.pids.clone();
                        } else {
                            shared_data
                                .in_backend_app_map
                                .insert(pkg.clone(), snapshot.clone());

                            if shared_data.config.global.is_debug {
                                if let Ok(mut log) = self.logger.lock() {
                                    log.debug(format!("进入Back:{}=>{:?}", pkg, snapshot));
                                }
                            }

                            shared_data.in_freezer_app_map.remove(pkg);
                            shared_data.in_dead_app_map.remove(pkg);
                        }
                    }
                    AppStatus::Dead => {
                        if shared_data.in_top_app.as_deref() == Some(pkg.as_str()) {
                            shared_data.in_top_app = None;
                        }

                        if !shared_data.in_dead_app_map.contains_key(pkg) {
                            shared_data
                                .in_dead_app_map
                                .insert(pkg.clone(), snapshot.clone());

                            if shared_data.config.global.is_debug {
                                if let Ok(mut log) = self.logger.lock() {
                                    log.debug(format!("进入Dead:{}=>{:?}", pkg, snapshot));
                                }
                            }

                            shared_data.in_backend_app_map.remove(pkg);
                            shared_data.in_freezer_app_map.remove(pkg);
                        }
                    }
                }
            }
        }
    }

    pub fn system_data_update(&mut self) {
        if let Ok(mut shared_data) = self.shared_data_lock.write() {
            shared_data.zram_usage_percent = system::get_zram_usage_percent();
            shared_data.is_screen = system::is_screen_on();
        }
    }

    pub fn start(&mut self) {
        loop {
            let scan_map = self.scan();
            self.category(scan_map);
            self.system_data_update();
        }
    }
}
