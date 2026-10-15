use crate::executor::actions::Action;
use crate::platform::cgroup;
use logger::Logger;

use crate::share::data::ShareData;
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::time::Instant;

pub struct Suppress {
    shared_data: Arc<RwLock<ShareData>>,
    executor_actions_receiver: mpsc::Receiver<Vec<Action>>,
    logger: Arc<Mutex<Logger>>,
}

impl Suppress {
    pub fn new(
        shared_data: Arc<RwLock<ShareData>>,
        executor_actions_receiver: mpsc::Receiver<Vec<Action>>,
        logger: Arc<Mutex<Logger>>,
    ) -> Self {
        Self {
            shared_data,
            executor_actions_receiver,
            logger,
        }
    }

    pub fn start(&mut self) {
        loop {
            let (interval, set, time) = match self.shared_data.read() {
                Ok(shared_data) => (
                    shared_data.config.global.shrink_cooldown,
                    shared_data.in_suppress_app_set.clone(),
                    shared_data.shrunk_cooldown,
                ),
                Err(_) => continue,
            };

            if interval < time.elapsed().as_secs() {
                for pkg in set {
                    let result = cgroup::shrink_anon(&pkg, true);
                    match result {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }

                if let Ok(mut share) = self.shared_data.write() {
                    share.shrunk_cooldown = Instant::now();
                };

                if let Ok(mut log) = self.logger.lock() {
                    log.info("周期性压制".to_string());
                }
            }

            let result = self.executor_actions_receiver.recv();

            match result {
                Ok(actions) => {
                    if let Ok(mut shared_data) = self.shared_data.write() {
                        for action in actions {
                            match action {
                                Action::OnShrinkAnon { pkg, pids } => {
                                    if shared_data.in_suppress_app_set.contains(&pkg) {
                                        continue;
                                    }
                                    let migrate_pids = cgroup::migrate_all_pids(&pkg, &pids);
                                    let result = cgroup::shrink_anon(&pkg, true);

                                    if let Ok(mut log) = self.logger.lock() {
                                        log.info(format!("迁移的pids:{:?}", migrate_pids));
                                    }

                                    match result {
                                        Ok(_) => {
                                            if let Ok(mut log) = self.logger.lock() {
                                                log.info(format!("压制:{}", pkg));
                                            }
                                            shared_data.in_suppress_app_set.insert(pkg);
                                        }
                                        Err(_) => {
                                            if let Ok(mut log) = self.logger.lock() {
                                                log.error("压制失败".to_string());
                                            }
                                        }
                                    }
                                }
                                Action::OnShrinkFile { pkg, pids } => {
                                    if shared_data.in_suppress_app_set.contains(&pkg) {
                                        continue;
                                    }

                                    let migrate_pids = cgroup::migrate_all_pids(&pkg, &pids);
                                    let result = cgroup::shrink_file(&pkg, true);

                                    if let Ok(mut log) = self.logger.lock() {
                                        log.info(format!("迁移的pids:{:?}", migrate_pids));
                                    }

                                    match result {
                                        Ok(_) => {
                                            if let Ok(mut log) = self.logger.lock() {
                                                log.info(format!("压制:{}", pkg));
                                            }
                                            shared_data.in_suppress_app_set.insert(pkg);
                                        }
                                        Err(_) => {
                                            if let Ok(mut log) = self.logger.lock() {
                                                log.error("压制失败".to_string());
                                            }
                                        }
                                    }
                                }
                                Action::OffShrink { pkg } => {
                                    if !shared_data.in_suppress_app_set.contains(&pkg) {
                                        continue;
                                    }

                                    // let result = cgroup::shrink_file(&pkg, false);

                                    shared_data.in_suppress_app_set.remove(&pkg);

                                    if let Ok(mut log) = self.logger.lock() {
                                        log.info(format!("解除压制:{}", pkg));
                                    }

                                    // match result {
                                    //     Ok(_) => {
                                    //         if let Ok(mut log) = self.logger.lock() {
                                    //             log.debug(format!("解除压制:{}", pkg));
                                    //         }
                                    //     }
                                    //     Err(_) => {
                                    //         if let Ok(mut log) = self.logger.lock() {
                                    //             log.error("解除压制失败".to_string());
                                    //         }
                                    //     }
                                    // }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Ok(mut log) = self.logger.lock() {
                        log.error(format!("获取Actions失败:{}", e));
                    }
                }
            }
        }
    }
}
