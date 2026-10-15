pub fn inotify_init(path: &str) -> inotify::Inotify {
    let inotify = inotify::Inotify::init().unwrap();
    inotify
        .watches()
        .add(path, inotify::WatchMask::ALL_EVENTS)
        .unwrap();
    inotify
}

pub fn inotify_blockage(inotify: &mut inotify::Inotify) {
    let mut buffer = [0u8; 4096];
    loop {
        match inotify.read_events(&mut buffer) {
            Ok(events) => {
                for _ in events {
                    break;
                }
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                panic!("读取 inotify 事件失败: {:?}", e);
            }
        }
    }
}
