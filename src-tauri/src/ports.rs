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

fn signal(pid: u32, sig: &str) -> bool {
    Command::new("/bin/kill")
        .args([sig, &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `kill -0`: true if the process exists and we may signal it.
fn alive(pid: u32) -> bool {
    signal(pid, "-0")
}

/// Kill gracefully: SIGTERM first so the process can flush/clean up,
/// escalate to SIGKILL only if it's still alive after ~500ms.
///
/// `port` guards against pid-reuse races: we re-check that this pid still
/// listens on this port before sending anything.
pub fn kill_pid_on_port(pid: u32, port: u16) -> Result<(), String> {
    if pid == 0 {
        return Err("invalid pid".into());
    }

    // Revalidate (pid, port) — the process may have exited since the list
    // was rendered and the pid could even have been reused.
    let still_listening = list_listening_ports()?
        .iter()
        .any(|e| e.pid == pid && e.port == port);
    if !still_listening {
        return Err(format!(
            "process {pid} no longer listens on port {port} — refresh the list"
        ));
    }

    if !signal(pid, "-TERM") {
        return Err(format!(
            "cannot signal pid {pid} (it may belong to another user or root)"
        ));
    }

    // Give it up to 500ms to exit cleanly
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if !alive(pid) {
            return Ok(());
        }
    }

    // Still alive — force kill
    signal(pid, "-KILL");
    std::thread::sleep(std::time::Duration::from_millis(50));
    if alive(pid) {
        Err(format!("pid {pid} survived SIGKILL"))
    } else {
        Ok(())
    }
}
