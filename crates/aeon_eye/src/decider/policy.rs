use crate::config;
use crate::executor::actions::Action;
use crate::share::app_snapshot::AppStatus;
use crate::share::data::ShareData;
use logger::Logger;
use std::sync::{Arc, RwLock};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::Duration;

pub struct Decider {
    shared_data: Arc<RwLock<ShareData>>,
    executor_actions_sender: mpsc::Sender<Vec<Action>>,
    logger: Arc<Mutex<Logger>>,
}

impl Decider {
    pub fn new(
        executor_actions_sender: mpsc::Sender<Vec<Action>>,
        shared_data: Arc<RwLock<ShareData>>,
        logger: Arc<Mutex<Logger>>,
    ) -> Self {
        Self {
            shared_data,
            executor_actions_sender,
            logger,
        }
    }

    pub fn start(&mut self) {
        loop {
            let decide_interval = match self.shared_data.read() {
                Ok(shared_data) => shared_data.config.global.decide_interval,
                Err(_) => return,
            };

            if decide_interval > 0 {
                thread::sleep(Duration::from_secs(decide_interval));
            }

            let shared_data = match self.shared_data.read() {
                Ok(shared_data) => shared_data,
                Err(_) => return,
            };

            let mut actions = Vec::new();

            for (pkg, snapshot) in &shared_data.in_freezer_app_map {
                let option = shared_data.config.strategies.get(pkg);
                match option {
                    Some(strategy) => {
                        if let AppStatus::Freezer { since } = snapshot.status
                            && strategy.freeze_timeout < since.elapsed().as_secs()
                        {
                            actions.push(Action::OnShrinkAnon {
                                pkg: pkg.clone(),
                                pids: snapshot.pids.clone(),
                            })
                        };
                    }
                    None => {
                        if let AppStatus::Freezer { since } = snapshot.status
                            && config::default_freeze_timeout() < since.elapsed().as_secs()
                        {
                            actions.push(Action::OnShrinkAnon {
                                pkg: pkg.clone(),
                                pids: snapshot.pids.clone(),
                            })
                        };
                    }
                }
            }

            for (pkg, snapshot) in &shared_data.in_backend_app_map {
                let option = shared_data.config.strategies.get(pkg);
                match option {
                    Some(strategy) => {
                        if let AppStatus::Back { since } = snapshot.status
                            && strategy.back_timeout < since.elapsed().as_secs()
                            && strategy.aggressive
                        {
                            actions.push(Action::OnShrinkAnon {
                                pkg: pkg.clone(),
                                pids: snapshot.pids.clone(),
                            })
                        };
                    }
                    None => {
                        /*if let AppStatus::Back { since } = snapshot.status
                            && config::default_back_timeout() < since.elapsed().as_secs()
                        {
                            actions.push(Action::OnShrinkAnon {
                                pkg: pkg.clone(),
                                pids: snapshot.pids.clone(),
                            })
                        };*/
                    }
                }
            }

            if let Some(pkg) = &shared_data.in_top_app {
                actions.push(Action::OffShrink { pkg: pkg.clone() })
            }

            let result = self.executor_actions_sender.send(actions);

            match result {
                Ok(_) => {}
                Err(e) => {
                    if let Ok(mut log) = self.logger.lock() {
                        log.error(format!("发送Actions失败:{}", e));
                    }
                }
            }
        }
    }
}
