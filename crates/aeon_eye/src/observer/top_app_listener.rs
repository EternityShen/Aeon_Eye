use crate::executor::actions::Action;
use crate::platform::system;
use crate::utils;
use inotify::Inotify;
use logger::Logger;
use std::sync::{Arc, Mutex, mpsc};

pub struct TopAppListener {
    executor_actions_sender: mpsc::Sender<Vec<Action>>,
    inotify: Inotify,
    logger: Arc<Mutex<Logger>>,
}

impl TopAppListener {
    pub fn new(
        executor_actions_sender: mpsc::Sender<Vec<Action>>,
        logger: Arc<Mutex<Logger>>,
    ) -> Self {
        let inotify = utils::inotify_init("/dev/cpuset/top-app");
        Self {
            executor_actions_sender,
            inotify,
            logger,
        }
    }

    pub fn start(&mut self) {
        loop {
            utils::inotify_blockage(&mut self.inotify);

            let option = system::get_top_package();

            if let Some(package) = option {
                let result = self
                    .executor_actions_sender
                    .send(vec![Action::OffShrink { pkg: package }]);

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        if let Ok(mut log) = self.logger.lock() {
                            log.error(format!("发送top-app失败:{}", e));
                        }
                    }
                }
            }
        }
    }
}
