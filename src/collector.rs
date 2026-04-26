use crate::models::{ProcessInfo, SystemSummary};
use anyhow::Result;

pub struct ProcessCollector;

impl ProcessCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self) -> Result<Vec<ProcessInfo>> {
        // Primary: macOS proc_listpids + proc_pidinfo syscalls
        // Fallback: parse `ps aux` output
        Self::collect_via_sysctl()
            .or_else(|_| Self::collect_via_ps())
    }

    fn collect_via_sysctl() -> Result<Vec<ProcessInfo>> {
        todo!("Implement proc_listpids + proc_pidinfo collection")
    }

    fn collect_via_ps() -> Result<Vec<ProcessInfo>> {
        todo!("Implement ps aux parsing fallback")
    }
}

pub fn get_system_summary() -> Result<SystemSummary> {
    todo!("Implement sysctl-based system summary (CPU, memory, swap)")
}
