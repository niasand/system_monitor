use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,
    pub user: String,
    pub command: String,
    pub cpu_percent: f64,
    pub rss_bytes: u64,
    pub state: ProcessState,
    pub elapsed_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProcessState {
    Running,
    Sleeping,
    Zombie,
    Stopped,
    Idle,
    Unknown,
}

impl ProcessState {
    pub fn from_char(c: char) -> Self {
        match c {
            'R' => Self::Running,
            'S' | 'D' => Self::Sleeping,
            'Z' => Self::Zombie,
            'T' => Self::Stopped,
            'I' => Self::Idle,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSummary {
    pub cpu_usage_percent: f64,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorOutput {
    pub summary: SystemSummary,
    pub top_cpu: Vec<ProcessInfo>,
    pub top_memory: Vec<ProcessInfo>,
    pub long_scripts: Vec<ProcessInfo>,
    pub zombies: Vec<ProcessInfo>,
}
