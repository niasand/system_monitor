use crate::memory_analyzer::MemoryAnalyzer;
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

    pub fn render(&self, output: &MonitorOutput, zombies: &[ZombieEntry]) -> String {
        if self.json_mode {
            self.render_json(output)
        } else {
            self.render_table(output, zombies)
        }
    }

    fn render_json(&self, output: &MonitorOutput) -> String {
        serde_json::to_string_pretty(output).unwrap_or_default()
    }

    fn render_table(&self, output: &MonitorOutput, zombies: &[ZombieEntry]) -> String {
        let mut lines = Vec::new();
        self.render_summary(&mut lines, &output.summary);
        lines.push(String::new());
        self.render_process_table(&mut lines, "Top CPU", &output.top_cpu, true);
        lines.push(String::new());
        self.render_process_table(&mut lines, "Top Memory", &output.top_memory, false);
        lines.push(String::new());
        self.render_scripts_table(&mut lines, &output.long_scripts);
        lines.push(String::new());
        self.render_zombie_table(&mut lines, zombies);
        lines.join("\n")
    }

    fn render_summary(&self, lines: &mut Vec<String>, s: &SystemSummary) {
        let header = self.bold("System Summary");
        lines.push(header);
        lines.push("─".repeat(60));

        let mem_used = MemoryAnalyzer::format_bytes(s.used_memory_bytes);
        let mem_total = MemoryAnalyzer::format_bytes(s.total_memory_bytes);
        let mem_pct = if s.total_memory_bytes > 0 {
            s.used_memory_bytes as f64 / s.total_memory_bytes as f64 * 100.0
        } else {
            0.0
        };

        let swap_used = MemoryAnalyzer::format_bytes(s.swap_used_bytes);
        let swap_total = MemoryAnalyzer::format_bytes(s.swap_total_bytes);

        let mem_line = format_bar("Memory", mem_pct, &format!("{mem_used} / {mem_total}"));
        let swap_pct = if s.swap_total_bytes > 0 {
            s.swap_used_bytes as f64 / s.swap_total_bytes as f64 * 100.0
        } else {
            0.0
        };
        let swap_line = format_bar("Swap  ", swap_pct, &format!("{swap_used} / {swap_total}"));

        lines.push(mem_line);
        lines.push(swap_line);
    }

    fn render_process_table(
        &self,
        lines: &mut Vec<String>,
        title: &str,
        processes: &[crate::models::ProcessInfo],
        is_cpu: bool,
    ) {
        lines.push(self.bold(title));
        lines.push("─".repeat(60));

        let header = format!(
            "{:<7} {:<8} {:>6} {:>10} {:<8} {}",
            "PID", "USER", if is_cpu { "%CPU" } else { "%MEM" }, "MEM", "STATE", "COMMAND"
        );
        lines.push(self.dim(&header));

        if processes.is_empty() {
            lines.push(self.dim("  (none)"));
            return;
        }

        for p in processes {
            let mem = MemoryAnalyzer::format_bytes(p.rss_bytes);
            let cpu = format!("{:.1}", p.cpu_percent);
            let value = if is_cpu { cpu } else { mem.clone() };

            let state_str = format!("{:?}", p.state);
            let cmd = truncate(&p.command, 30);

            let cpu_colored = if p.cpu_percent > 50.0 {
                self.red(&value)
            } else if p.cpu_percent > 20.0 {
                self.yellow(&value)
            } else {
                value.clone()
            };

            lines.push(format!(
                "{:<7} {:<8} {:>6} {:>10} {:<8} {}",
                p.pid, p.user, cpu_colored, mem, state_str, cmd
            ));
        }
    }

    fn render_scripts_table(
        &self,
        lines: &mut Vec<String>,
        scripts: &[crate::models::ProcessInfo],
    ) {
        lines.push(self.bold("Long-Running Scripts (> 12h)"));
        lines.push("─".repeat(60));

        let header = format!(
            "{:<7} {:<8} {:>10} {:>12} {}",
            "PID", "USER", "ELAPSED", "MEM", "COMMAND"
        );
        lines.push(self.dim(&header));

        if scripts.is_empty() {
            lines.push(self.dim("  (none)"));
            return;
        }

        for p in scripts {
            let elapsed = format_elapsed(p.elapsed_secs);
            let mem = MemoryAnalyzer::format_bytes(p.rss_bytes);
            let cmd = truncate(&p.command, 30);
            lines.push(format!(
                "{:<7} {:<8} {:>10} {:>12} {}",
                p.pid, p.user, elapsed, mem, cmd
            ));
        }
    }

    fn render_zombie_table(&self, lines: &mut Vec<String>, zombies: &[ZombieEntry]) {
        lines.push(self.bold("Zombie Processes"));
        lines.push("─".repeat(60));

        let header = format!(
            "{:<7} {:<8} {:>12} {:<20} {}",
            "PID", "USER", "PPID", "PARENT", "COMMAND"
        );
        lines.push(self.dim(&header));

        if zombies.is_empty() {
            lines.push(self.dim("  (none)"));
            return;
        }

        for z in zombies {
            let cmd = truncate(&z.process.command, 25);
            let parent = truncate(&z.parent_command, 18);
            let line = format!(
                "{:<7} {:<8} {:>12} {:<20} {}",
                z.process.pid, z.process.user, z.parent_pid, parent, cmd
            );
            lines.push(if self.use_color {
                "\x1b[31m".to_string() + &line + "\x1b[0m"
            } else {
                line
            });
        }
    }

    // Color helpers — only emit ANSI when use_color is true
    fn bold(&self, s: &str) -> String {
        if self.use_color {
            format!("\x1b[1m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn dim(&self, s: &str) -> String {
        if self.use_color {
            format!("\x1b[2m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn red(&self, s: &str) -> String {
        if self.use_color {
            format!("\x1b[31m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn yellow(&self, s: &str) -> String {
        if self.use_color {
            format!("\x1b[33m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
}

fn format_bar(label: &str, pct: f64, detail: &str) -> String {
    let bar_width = 20;
    let filled = (pct / 100.0 * bar_width as f64).round() as usize;
    let filled = filled.min(bar_width);
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);
    format!("{label}: [{bar}] {pct:5.1}%  {detail}")
}

fn format_elapsed(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max.saturating_sub(1);
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    format!("{}…", &s[..end])
}
