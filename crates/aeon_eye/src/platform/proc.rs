use std::{collections::HashMap, fs};

pub fn find_pids_by_package(pkg: &str) -> Vec<i32> {
    let mut pids = Vec::new();
    let prefix = format!("{}:", pkg);

    let entries = match fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return pids,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let pid = match name.to_string_lossy().parse::<i32>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if let Some(cmd) = get_cmdline(pid) {
            if cmd == pkg || cmd.starts_with(&prefix) {
                pids.push(pid);
            }
        }
    }

    pids
}

pub fn find_pids_for_packages(packages: &[String]) -> HashMap<String, Vec<i32>> {
    let mut result: HashMap<String, Vec<i32>> =
        packages.iter().map(|p| (p.clone(), Vec::new())).collect();

    let entries = match fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let pid = match name.to_string_lossy().parse::<i32>() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let cmd = match get_cmdline(pid) {
            Some(c) => c,
            None => continue,
        };

        // 取主包名部分（去掉 :xxx 后缀）
        let base = cmd.split(':').next().unwrap_or(&cmd);

        if let Some(pids) = result.get_mut(base) {
            pids.push(pid);
        }
    }

    result
}

pub fn get_cmdline(pid: i32) -> Option<String> {
    let content = fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    let first = content.split(|&b| b == 0).next()?;
    if first.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(first).into_owned())
}

pub fn get_adj(pid: i32) -> Option<i32> {
    let content = fs::read_to_string(format!("/proc/{}/oom_score_adj", pid)).ok()?;
    content.trim().parse::<i32>().ok()
}

pub fn get_min_adj(pids: &[i32]) -> Option<i32> {
    pids.iter().filter_map(|pid| get_adj(*pid)).min()
}

pub fn get_uid(pid: i32) -> Option<u32> {
    let content = fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let real_uid = rest.split_whitespace().next()?;
            return real_uid.parse::<u32>().ok();
        }
    }
    None
}
