use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    path::Path,
};

pub fn memcg_path(pkg: &str) -> Option<String> {
    let active = format!("/dev/memcg/apps/{}", pkg);
    if std::path::Path::new(&active).exists() {
        return Some(active);
    }
    let inactive = format!("/dev/memcg/apps/inactive_apps/{}", pkg);
    if std::path::Path::new(&inactive).exists() {
        return Some(inactive);
    }
    None
}

pub fn freeze_path(uid: u32) -> String {
    format!("/sys/fs/cgroup/apps/uid_{}/cgroup.freeze", uid)
}

pub fn get_uid_by_package(pkg: &str) -> Result<u32, String> {
    let content = fs::read_to_string("/data/system/packages.list")
        .map_err(|e| format!("读取 packages.list 失败: {}", e))?;

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        if let Some(name) = parts.next() {
            if name == pkg {
                if let Some(uid_str) = parts.next() {
                    return uid_str
                        .parse::<u32>()
                        .map_err(|_| format!("UID 解析失败: {}", uid_str));
                }
            }
        }
    }

    Err(format!("包名 {} 未找到", pkg))
}

/// 判断应用是否被墓碑冻结。
///
/// 墓碑的实现方式：为每个进程建一个子 cgroup，
/// 冻结时把该子目录的 `cgroup.freeze` 写为 1。
/// UID 根目录的 cgroup.freeze 永远是 0。
///
/// 判定规则：只要 UID 下任意一个子 cgroup 被冻结，就算整个应用被冻结。
pub fn is_frozen(pkg: &str) -> bool {
    let uid = match get_uid_by_package(pkg) {
        Ok(u) => u,
        Err(_) => return false,
    };

    let root = format!("/sys/fs/cgroup/apps/uid_{}", uid);
    check_freeze_recursive(Path::new(&root))
}

pub fn is_frozen_uid(uid: u32) -> bool {
    let root = format!("/sys/fs/cgroup/apps/uid_{}", uid);
    check_freeze_recursive(Path::new(&root))
}

/// 递归查找目录及其子目录下是否存在 cgroup.freeze = 1。
fn check_freeze_recursive(dir: &Path) -> bool {
    // 先看当前目录
    let freeze_file = dir.join("cgroup.freeze");
    if let Ok(s) = std::fs::read_to_string(&freeze_file) {
        if s.trim() == "1" {
            return true;
        }
    }

    // 递归子目录，找到一个就返回
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && check_freeze_recursive(&path) {
                return true;
            }
        }
    }

    false
}

/// 读取该应用占用的物理内存（字节）。
pub fn read_mem_usage(pkg: &str) -> u64 {
    let path = match memcg_path(pkg) {
        Some(p) => format!("{}/memory.usage_in_bytes", p),
        None => return 0,
    };
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// 解析 memory.swap_stat 中指定的字段（单位 KB → 字节）。
fn parse_swap_stat(path: &str, field_name: &str) -> u64 {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    for line in content.lines() {
        let line = line.trim();
        // 格式：zramCompressedSize:    17810 KB
        if let Some(rest) = line.strip_prefix(field_name) {
            let rest = rest.trim_start_matches(':').trim();
            // 取第一个数字（KB）
            if let Some(num_str) = rest.split_whitespace().next() {
                if let Ok(kb) = num_str.parse::<u64>() {
                    return kb * 1024;
                }
            }
        }
    }
    0
}

/// 读取 ZRAM 压缩后的大小（字节）。
pub fn read_zram_usage(pkg: &str) -> u64 {
    let path = match memcg_path(pkg) {
        Some(p) => format!("{}/memory.swap_stat", p),
        None => return 0,
    };
    parse_swap_stat(&path, "zramCompressedSize")
}

/// 读取 ZRAM 压缩前的原始大小（字节）。
pub fn read_zram_original(pkg: &str) -> u64 {
    let path = match memcg_path(pkg) {
        Some(p) => format!("{}/memory.swap_stat", p),
        None => return 0,
    };
    parse_swap_stat(&path, "zramOrignalSize")
}

/// 读取该应用 cgroup 内的所有 PID。
pub fn read_cgroup_procs(pkg: &str) -> Vec<i32> {
    let path = match memcg_path(pkg) {
        Some(p) => format!("{}/cgroup.procs", p),
        None => return Vec::new(),
    };
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| line.trim().parse::<i32>().ok())
        .collect()
}

/// 向只写 cgroup 文件写入内容。
///
/// 注意：cgroup 文件不可读、不可创建，必须用 OpenOptions 只写打开。
fn write_cgroup_file(path: &str, value: &str) -> io::Result<()> {
    let mut f = fs::OpenOptions::new().write(true).open(path)?;
    f.write_all(value.as_bytes())
}

/// 开启 / 关闭匿名页强制回收。
pub fn shrink_anon(pkg: &str, on: bool) -> io::Result<()> {
    let path = match memcg_path(pkg) {
        Some(p) => format!("{}/memory.force_shrink_anon", p),
        None => return Err(io::Error::new(io::ErrorKind::NotFound, "cgroup 路径不存在")),
    };
    write_cgroup_file(&path, if on { "1" } else { "0" })
}

/// 开启 / 关闭文件页强制回收。
pub fn shrink_file(pkg: &str, on: bool) -> io::Result<()> {
    let path = match memcg_path(pkg) {
        Some(p) => format!("{}/memory.force_shrink_file", p),
        None => return Err(io::Error::new(io::ErrorKind::NotFound, "cgroup 路径不存在")),
    };
    write_cgroup_file(&path, if on { "1" } else { "0" })
}

/// 把指定 PID 迁移到该应用的 cgroup 里。
pub fn migrate_pid(pkg: &str, pid: i32) -> io::Result<()> {
    let path = match memcg_path(pkg) {
        Some(p) => format!("{}/cgroup.procs", p),
        None => return Err(io::Error::new(io::ErrorKind::NotFound, "cgroup 路径不存在")),
    };
    write_cgroup_file(&path, &pid.to_string())
}

/// 批量迁移所有 PID，跳过已死进程，返回成功迁移的列表。
pub fn migrate_all_pids(pkg: &str, pids: &[i32]) -> Vec<i32> {
    let mut migrated = Vec::new();

    for &pid in pids {
        match migrate_pid(pkg, pid) {
            Ok(_) => migrated.push(pid),
            // ESRCH = 3：进程已死，跳过
            Err(e) if e.raw_os_error() == Some(3) => continue,
            Err(_) => continue,
        }
    }

    migrated
}

/// 批量迁移 PID，每批最多 batch_size 个。
///
/// 返回本轮实际迁移成功的 PID 列表。
/// 忽略 ESRCH（进程已死）错误。
pub fn migrate_pids_batch(pkg: &str, pids: &[i32], batch_size: usize) -> Vec<i32> {
    let mut migrated = Vec::new();

    for &pid in pids.iter().take(batch_size) {
        match migrate_pid(pkg, pid) {
            Ok(_) => migrated.push(pid),
            Err(e) => {
                // ESRCH = 3，进程已死，忽略
                if e.raw_os_error() == Some(3) {
                    continue;
                }
                // 其他错误也忽略，但不加入成功列表
            }
        }
    }

    migrated
}

/// 一次性扫描 /sys/fs/cgroup/apps/，返回所有"至少有一个子 cgroup 被冻结"的 UID。
pub fn scan_frozen_uids() -> HashSet<u32> {
    let mut result = HashSet::new();

    let entries = match std::fs::read_dir("/sys/fs/cgroup/apps") {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.flatten() {
        let uid_dir = entry.path();
        if !uid_dir.is_dir() {
            continue;
        }

        let name = uid_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let uid = match name
            .strip_prefix("uid_")
            .and_then(|s| s.parse::<u32>().ok())
        {
            Some(u) => u,
            None => continue,
        };

        if check_freeze_recursive(&uid_dir) {
            result.insert(uid);
        }
    }

    result
}
