use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::models::{ProcessInfo, ProcessState, SystemSummary};

pub struct ProcessCollector;

impl ProcessCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self) -> Result<Vec<ProcessInfo>> {
        Self::collect_via_ps()
    }

    fn collect_via_ps() -> Result<Vec<ProcessInfo>> {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,uid=,user=,%cpu=,rss=,stat=,etime=,command="])
            .output()
            .context("failed to execute ps")?;

        if !output.status.success() {
            bail!("ps exited with {}", output.status);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().filter_map(parse_ps_line).collect())
    }
}

fn parse_ps_line(line: &str) -> Option<ProcessInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        return None;
    }

    Some(ProcessInfo {
        pid: parts[0].parse().ok()?,
        ppid: parts[1].parse().ok()?,
        uid: parts[2].parse().ok()?,
        user: parts[3].to_string(),
        cpu_percent: parts[4].parse().ok()?,
        rss_bytes: parts[5].parse::<u64>().ok()? * 1024, // ps reports RSS in KB
        state: ProcessState::from_char(parts[6].chars().next()?),
        elapsed_secs: parse_elapsed(parts[7])?,
        command: parts[8..].join(" "),
    })
}

fn parse_elapsed(s: &str) -> Option<u64> {
    let s = s.trim();
    let (days, time_part) = if let Some((d, t)) = s.split_once('-') {
        (d.parse::<u64>().ok()?, t)
    } else {
        (0, s)
    };

    let segs: Vec<u64> = time_part.split(':').filter_map(|p| p.parse().ok()).collect();
    let secs = match segs.len() {
        2 => segs[0] * 60 + segs[1],
        3 => segs[0] * 3600 + segs[1] * 60 + segs[2],
        _ => return None,
    };

    Some(days * 86400 + secs)
}

// --- System Summary ---

pub fn get_system_summary() -> Result<SystemSummary> {
    let total = total_memory();
    let page_size = page_size();
    let (free, _active, inactive, _wired) = vm_page_counts();
    let used = total.saturating_sub((free + inactive) as u64 * page_size as u64);
    let (swap_total, swap_used) = swap_info();
    let cpu_usage = cpu_usage_percent();

    Ok(SystemSummary {
        cpu_usage_percent: cpu_usage,
        total_memory_bytes: total,
        used_memory_bytes: used,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
    })
}

fn cpu_usage_percent() -> f64 {
    let output = Command::new("top")
        .args(["-l", "2", "-n", "0", "-s", "1"])
        .output()
        .ok();

    let Some(stdout) = output.map(|o| String::from_utf8_lossy(&o.stdout).to_string()) else {
        return 0.0;
    };

    // Take the second "CPU usage:" line (first is cumulative since boot)
    let mut cpu_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("CPU usage:"))
        .collect();

    let line = cpu_lines.pop().unwrap_or("");
    // Format: "CPU usage: 12.5% user, 8.3% sys, 79.2% idle"
    let mut user_pct = 0.0_f64;
    let mut sys_pct = 0.0_f64;
    for part in line.split(',') {
        let part = part.trim();
        if part.ends_with("user") {
            user_pct = part.trim_end_matches("user").trim().trim_end_matches('%').parse().unwrap_or(0.0);
        } else if part.ends_with("sys") {
            sys_pct = part.trim_end_matches("sys").trim().trim_end_matches('%').parse().unwrap_or(0.0);
        }
    }
    user_pct + sys_pct
}

fn total_memory() -> u64 {
    let mut val: u64 = 0;
    let mut size = std::mem::size_of::<u64>();
    let mut mib: [i32; 2] = [CTL_HW, HW_MEMSIZE];
    unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            2,
            &mut val as *mut _ as *mut _,
            &mut size as *mut _,
            std::ptr::null_mut(),
            0,
        );
    }
    val
}

fn page_size() -> i32 {
    let mut val: i32 = 0;
    let mut size = std::mem::size_of::<i32>();
    let mut mib: [i32; 2] = [CTL_HW, HW_PAGESIZE];
    unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            2,
            &mut val as *mut _ as *mut _,
            &mut size as *mut _,
            std::ptr::null_mut(),
            0,
        );
    }
    val
}

fn vm_page_counts() -> (u32, u32, u32, u32) {
    let Some(output) = Command::new("vm_stat").output().ok() else {
        return (0, 0, 0, 0);
    };
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut free = 0u32;
    let mut active = 0u32;
    let mut inactive = 0u32;
    let mut wired = 0u32;

    for line in stdout.lines() {
        if let Some(v) = vm_stat_value(line, "Pages free") {
            free = v;
        }
        if let Some(v) = vm_stat_value(line, "Pages active") {
            active = v;
        }
        if let Some(v) = vm_stat_value(line, "Pages inactive") {
            inactive = v;
        }
        if let Some(v) = vm_stat_value(line, "Pages wired down") {
            wired = v;
        }
    }

    (free, active, inactive, wired)
}

fn vm_stat_value(line: &str, prefix: &str) -> Option<u32> {
    if line.starts_with(prefix) {
        line.split(':')
            .nth(1)?
            .trim()
            .replace('.', "")
            .parse()
            .ok()
    } else {
        None
    }
}

fn swap_info() -> (u64, u64) {
    let Some(output) = Command::new("sysctl")
        .args(["-n", "vm.swapusage"])
        .output()
        .ok()
    else {
        return (0, 0);
    };
    let stdout = String::from_utf8_lossy(&output.stdout);

    let total = swap_field(&stdout, "total");
    let used = swap_field(&stdout, "used");
    (total, used)
}

fn swap_field(output: &str, key: &str) -> u64 {
    let pattern = format!("{key} = ");
    let Some(start) = output.find(&pattern) else {
        return 0;
    };
    let rest = &output[start + pattern.len()..];
    let raw = rest.split_whitespace().next().unwrap_or("0");
    parse_size(raw)
}

fn parse_size(s: &str) -> u64 {
    if s.ends_with('G') {
        (s.trim_end_matches('G').parse::<f64>().unwrap_or(0.0) * 1_073_741_824.0) as u64
    } else if s.ends_with('M') {
        (s.trim_end_matches('M').parse::<f64>().unwrap_or(0.0) * 1_048_576.0) as u64
    } else if s.ends_with('K') {
        (s.trim_end_matches('K').parse::<f64>().unwrap_or(0.0) * 1024.0) as u64
    } else {
        s.parse().unwrap_or(0)
    }
}

// macOS sysctl constants
const CTL_HW: i32 = 6;
const HW_MEMSIZE: i32 = 24;
const HW_PAGESIZE: i32 = 7;

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_elapsed ---

    #[test]
    fn elapsed_mm_ss() {
        assert_eq!(parse_elapsed("05:30"), Some(330));
    }

    #[test]
    fn elapsed_hh_mm_ss() {
        assert_eq!(parse_elapsed("02:30:45"), Some(9045));
    }

    #[test]
    fn elapsed_dd_hh_mm_ss() {
        assert_eq!(parse_elapsed("3-12:00:00"), Some(3 * 86400 + 43200));
    }

    #[test]
    fn elapsed_zero() {
        assert_eq!(parse_elapsed("00:00"), Some(0));
    }

    #[test]
    fn elapsed_single_digit() {
        assert_eq!(parse_elapsed("1:23"), Some(83));
    }

    #[test]
    fn elapsed_with_whitespace() {
        assert_eq!(parse_elapsed("  05:30  "), Some(330));
    }

    #[test]
    fn elapsed_invalid_returns_none() {
        assert_eq!(parse_elapsed("invalid"), None);
        assert_eq!(parse_elapsed(""), None);
    }

    #[test]
    fn elapsed_large_days() {
        assert_eq!(parse_elapsed("30-00:00:01"), Some(30 * 86400 + 1));
    }

    // --- parse_size ---

    #[test]
    fn size_gb() {
        assert_eq!(parse_size("2.0G"), 2_147_483_648);
    }

    #[test]
    fn size_mb() {
        assert_eq!(parse_size("512.0M"), 536_870_912);
    }

    #[test]
    fn size_kb() {
        assert_eq!(parse_size("2048K"), 2_097_152);
    }

    #[test]
    fn size_plain_bytes() {
        assert_eq!(parse_size("1024"), 1024);
    }

    // --- vm_stat_value ---

    #[test]
    fn vm_stat_extracts_number() {
        assert_eq!(vm_stat_value("Pages free:         1234567.", "Pages free"), Some(1234567));
    }

    #[test]
    fn vm_stat_wrong_prefix() {
        assert_eq!(vm_stat_value("Pages active:  100.", "Pages free"), None);
    }
}
