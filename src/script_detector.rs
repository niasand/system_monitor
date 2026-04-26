use crate::models::ProcessInfo;

const INTERPRETERS: &[&str] = &[
    "bash", "sh", "zsh", "dash", "fish",
    "python", "python3", "python2",
    "node", "ruby", "perl", "php",
];

const DEFAULT_THRESHOLD_SECS: u64 = 12 * 60 * 60; // 12 hours

pub struct ScriptDetector {
    pub threshold_secs: u64,
    pub current_uid: u32,
}

impl ScriptDetector {
    pub fn new(current_uid: u32) -> Self {
        Self {
            threshold_secs: DEFAULT_THRESHOLD_SECS,
            current_uid,
        }
    }

    pub fn with_threshold(mut self, secs: u64) -> Self {
        self.threshold_secs = secs;
        self
    }

    pub fn detect(&self, processes: &[ProcessInfo]) -> Vec<ProcessInfo> {
        processes
            .iter()
            .filter(|p| self.is_user_script(p))
            .cloned()
            .collect()
    }

    fn is_user_script(&self, p: &ProcessInfo) -> bool {
        p.uid == self.current_uid
            && p.elapsed_secs > self.threshold_secs
            && Self::is_interpreter(&p.command)
    }

    fn is_interpreter(command: &str) -> bool {
        let cmd_lower = command.to_lowercase();
        INTERPRETERS.iter().any(|interp| {
            cmd_lower == *interp || cmd_lower.starts_with(&format!("{interp}-"))
        })
    }
}
