use crate::models::{ProcessInfo, ProcessState};

pub struct ZombieDetector;

impl ZombieDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect(&self, processes: &[ProcessInfo]) -> Vec<ZombieEntry> {
        let zombies: Vec<&ProcessInfo> = processes
            .iter()
            .filter(|p| p.state == ProcessState::Zombie)
            .collect();

        zombies
            .into_iter()
            .map(|z| {
                let parent = processes.iter().find(|p| p.pid == z.ppid);
                ZombieEntry {
                    process: z.clone(),
                    parent_pid: z.ppid,
                    parent_command: parent.map(|p| p.command.clone()).unwrap_or_default(),
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ZombieEntry {
    pub process: ProcessInfo,
    pub parent_pid: i32,
    pub parent_command: String,
}
