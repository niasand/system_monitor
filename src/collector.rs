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

    Ok(SystemSummary {
        cpu_usage_percent: 0.0,
        total_memory_bytes: total,
        used_memory_bytes: used,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
    })
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
