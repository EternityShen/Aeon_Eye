use std::fs;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 读取 ZRAM 使用率（百分比）。
pub fn get_zram_usage_percent() -> f64 {
    let content = match fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return 0.0,
    };

    let mut swap_total: u64 = 0;
    let mut swap_free: u64 = 0;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("SwapTotal:") {
            swap_total = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("SwapFree:") {
            swap_free = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    if swap_total == 0 {
        return 0.0;
    }

    let used = swap_total.saturating_sub(swap_free);
    (used as f64 / swap_total as f64) * 100.0
}

/// 读取 1 分钟平均负载。
pub fn get_cpu_load_1min() -> f64 {
    fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// 读取 CPU 核心数。
pub fn get_cpu_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// 判断 CPU 是否繁忙。
pub fn is_cpu_busy(ratio: f32) -> bool {
    let cores = get_cpu_cores() as f64;
    let load = get_cpu_load_1min();
    load > cores * ratio as f64
}

/// 判断屏幕是否点亮。
pub fn is_screen_on() -> bool {
    let result = Command::new("sh")
        .arg("-c")
        .arg("dumpsys power | grep -E \"mHoldingDisplaySuspendBlocker|mScreenOn\"")
        .output()
        .expect("Failed to execute command");
    let output = String::from_utf8_lossy(&result.stdout);
    output.contains("true")
}

// ============================================================
// 顶层应用缓存
// ============================================================

struct TopPkgCache {
    value: Option<String>,
    updated_at: Instant,
}

static TOP_PKG_CACHE: Mutex<Option<TopPkgCache>> = Mutex::new(None);
const TOP_PKG_TTL: Duration = Duration::from_secs(5);

/// 获取当前顶层应用包名（带 5 秒缓存）。
pub fn get_top_package() -> Option<String> {
    {
        let cache = TOP_PKG_CACHE.lock().ok()?;
        if let Some(c) = cache.as_ref() {
            if c.updated_at.elapsed() < TOP_PKG_TTL {
                return c.value.clone();
            }
        }
    }
    refresh_top_package()
}

/// 强制刷新顶层应用缓存，立即调用 dumpsys。
pub fn refresh_top_package() -> Option<String> {
    let pkg = query_top_package_from_dumpsys();

    if let Ok(mut cache) = TOP_PKG_CACHE.lock() {
        *cache = Some(TopPkgCache {
            value: pkg.clone(),
            updated_at: Instant::now(),
        });
    }

    pkg
}

/// 实际执行 dumpsys 并解析包名。
fn query_top_package_from_dumpsys() -> Option<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("dumpsys window displays | grep mCurrentFocus")
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);

    // 形如：
    //   mCurrentFocus=Window{a1b2c3 u0 com.tencent.mm/com.tencent.mm.ui.LauncherUI}
    // 解析出 u0 后面的包名部分
    let idx = text.find(" u0 ")?;
    let rest = &text[idx + 4..];
    let end = rest.find('/')?;
    let pkg = &rest[..end];

    if pkg.is_empty() {
        None
    } else {
        Some(pkg.to_string())
    }
}
