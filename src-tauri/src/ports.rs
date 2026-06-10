use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct PortEntry {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub address: String,
    /// Resident set size in KB (0 if unknown)
    pub memory_kb: u64,
}

/// RSS (KB) for a set of pids, in one `ps` call.
fn rss_map(pids: &[u32]) -> HashMap<u32, u64> {
    let mut map = HashMap::new();
    if pids.is_empty() {
        return map;
    }
    let list = pids
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");
    if let Ok(out) = Command::new("/bin/ps")
        .args(["-o", "pid=,rss=", "-p", &list])
        .output()
    {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut it = line.split_whitespace();
            if let (Some(pid), Some(rss)) = (it.next(), it.next()) {
                if let (Ok(pid), Ok(rss)) = (pid.parse(), rss.parse()) {
                    map.insert(pid, rss);
                }
            }
        }
    }
    map
}

/// Parent pid, or None for pid 0/1 or on failure.
pub fn parent_pid(pid: u32) -> Option<u32> {
    let out = Command::new("/bin/ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let ppid: u32 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    if ppid <= 1 {
        None
    } else {
        Some(ppid)
    }
}

/// List all TCP ports in LISTEN state via `lsof`.
///
/// Uses lsof's machine-readable output (`-F pcn`) so process names
/// containing spaces are parsed correctly:
///   p<pid>     — starts a process block
///   c<command> — process name
///   n<name>    — network name, e.g. "*:3000" or "127.0.0.1:5173"
pub fn list_listening_ports() -> Result<Vec<PortEntry>, String> {
    // Absolute path: GUI apps launched from Finder get a minimal PATH
    let output = Command::new("/usr/sbin/lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcn"])
        .output()
        .map_err(|e| format!("failed to run lsof: {e}"))?;

    // lsof exits non-zero when nothing matches; treat that as an empty list
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() && stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let mut seen: HashSet<(u32, u16)> = HashSet::new();
    let mut cur_pid: u32 = 0;
    let mut cur_name = String::new();

    for line in stdout.lines() {
        let (tag, value) = match line.split_at_checked(1) {
            Some(parts) => parts,
            None => continue,
        };
        match tag {
            "p" => {
                cur_pid = value.parse().unwrap_or(0);
                cur_name.clear();
            }
            "c" => cur_name = value.to_string(),
            "n" => {
                if cur_pid == 0 {
                    continue;
                }
                // value looks like "*:3000", "127.0.0.1:5173" or "[::1]:8080"
                if let Some(idx) = value.rfind(':') {
                    let (addr, port_str) = value.split_at(idx);
                    if let Ok(port) = port_str[1..].parse::<u16>() {
                        // Dedupe IPv4/IPv6 duplicates of the same pid+port
                        if seen.insert((cur_pid, port)) {
                            entries.push(PortEntry {
                                port,
                                pid: cur_pid,
                                process_name: cur_name.clone(),
                                address: addr.to_string(),
                                memory_kb: 0,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Fill in memory usage with a single ps call
    let mut pids: Vec<u32> = entries.iter().map(|e| e.pid).collect();
    pids.sort_unstable();
    pids.dedup();
    let rss = rss_map(&pids);
    for e in &mut entries {
        e.memory_kb = rss.get(&e.pid).copied().unwrap_or(0);
    }

    Ok(entries)
}

/// Kill a process with SIGKILL.
pub fn kill_pid(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("invalid pid".into());
    }
    let status = Command::new("/bin/kill")
        .args(["-9", &pid.to_string()])
        .status()
        .map_err(|e| format!("failed to run kill: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "kill failed for pid {pid} (it may require higher privileges or already exited)"
        ))
    }
}
