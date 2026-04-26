use crate::models::ProcessInfo;

pub struct CpuAnalyzer {
    pub top_n: usize,
}

impl CpuAnalyzer {
    pub fn new(top_n: usize) -> Self {
        Self { top_n }
    }

    pub fn analyze(&self, processes: &[ProcessInfo]) -> Vec<ProcessInfo> {
        let mut sorted: Vec<&ProcessInfo> = processes.iter().collect();
        sorted.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(self.top_n).cloned().collect()
    }
}
