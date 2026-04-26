use crate::models::{MonitorOutput, SystemSummary};
use crate::zombie_detector::ZombieEntry;

pub struct Renderer {
    pub use_color: bool,
    pub json_mode: bool,
}

impl Renderer {
    pub fn new(use_color: bool, json_mode: bool) -> Self {
        Self { use_color, json_mode }
    }

    pub fn render(
        &self,
        output: &MonitorOutput,
        zombies: &[ZombieEntry],
    ) -> String {
        if self.json_mode {
            self.render_json(output, zombies)
        } else {
            self.render_table(output, zombies)
        }
    }

    fn render_json(&self, output: &MonitorOutput, zombies: &[ZombieEntry]) -> String {
        let _ = zombies;
        serde_json::to_string_pretty(output).unwrap_or_default()
    }

    fn render_table(&self, output: &MonitorOutput, zombies: &[ZombieEntry]) -> String {
        let mut lines = Vec::new();
        self.render_summary(&mut lines, &output.summary);
        self.render_process_table(&mut lines, "Top CPU", &output.top_cpu);
        self.render_process_table(&mut lines, "Top Memory", &output.top_memory);
        self.render_process_table(&mut lines, "Long-Running Scripts", &output.long_scripts);
        self.render_zombie_table(&mut lines, zombies);
        lines.join("\n")
    }

    fn render_summary(&self, lines: &mut Vec<String>, summary: &SystemSummary) {
        let _ = (lines, summary);
        todo!("Render system summary header")
    }

    fn render_process_table(&self, lines: &mut Vec<String>, title: &str, processes: &[crate::models::ProcessInfo]) {
        let _ = (lines, title, processes);
        todo!("Render process table")
    }

    fn render_zombie_table(&self, lines: &mut Vec<String>, zombies: &[ZombieEntry]) {
        let _ = (lines, zombies);
        todo!("Render zombie table")
    }
}
