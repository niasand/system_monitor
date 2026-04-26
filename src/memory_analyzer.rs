use crate::models::ProcessInfo;

pub struct MemoryAnalyzer {
    pub top_n: usize,
}

impl MemoryAnalyzer {
    pub fn new(top_n: usize) -> Self {
        Self { top_n }
    }

    pub fn analyze(&self, processes: &[ProcessInfo]) -> Vec<ProcessInfo> {
        let mut sorted: Vec<&ProcessInfo> = processes.iter().collect();
        sorted.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));
        sorted.into_iter().take(self.top_n).cloned().collect()
    }

    pub fn format_bytes(bytes: u64) -> String {
        const GB: u64 = 1_073_741_824;
        const MB: u64 = 1_048_576;
        const KB: u64 = 1_024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else {
            format!("{} KB", bytes / KB)
        }
    }
}
