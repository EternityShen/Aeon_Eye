use aeon_eye::config::Config;
use aeon_eye::decider;
use aeon_eye::executor;
use aeon_eye::observer;
use aeon_eye::observer::top_app_listener;
use aeon_eye::share::data::ShareData;
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;

const CONFIG_PATH: &str = "/data/adb/modules/ShenAeonEye/config.toml";
const LOG_PATH: &str = "/data/adb/modules/ShenAeonEye/log.log";

fn main() {
    let con = std::fs::read_to_string(&CONFIG_PATH).unwrap();
    let config: Config = toml::from_str(&con).unwrap();

    let mut log = logger::Logger::new(LOG_PATH);
    log.clear();

    log.info("你好丫!我是ShenEternity(沈之永恒)".to_string());
    log.info("本模块并非通用模块,可能只有一加和指定内核版本可用".to_string());

    let logger = Arc::new(Mutex::new(log));

    let share_data = Arc::new(RwLock::new(ShareData::new(Arc::new(config))));

    let (tx, rx) = mpsc::channel();

    let mut observer_updater =
        observer::share_data_updater::SharedUpdater::new(share_data.clone(), logger.clone());
    let mut decider_policy =
        decider::policy::Decider::new(tx.clone(), share_data.clone(), logger.clone());
    let mut executor = executor::runner::Suppress::new(share_data, rx, logger.clone());
    let mut top_app_listener = top_app_listener::TopAppListener::new(tx, logger);

    let _ = thread::Builder::new()
        .name("observer_updater".to_string())
        .spawn(move || {
            observer_updater.start();
        });

    let _ = thread::Builder::new()
        .name("decider_policy".to_string())
        .spawn(move || {
            decider_policy.start();
        });

    let _ = thread::Builder::new()
        .name("top_app_listener".to_string())
        .spawn(move || {
            top_app_listener.start();
        });

    executor.start();
}
