use crate::memory_analyzer::MemoryAnalyzer;
use crate::models::MonitorOutput;
use crate::zombie_detector::ZombieEntry;

pub struct FeishuRenderer;

impl FeishuRenderer {
    pub fn build_message(
        output: &MonitorOutput,
        zombies: &[ZombieEntry],
        hostname: &str,
        timestamp: &str,
    ) -> serde_json::Value {
        let mut elements = Vec::new();

        elements.push(summary_section(&output.summary));
        elements.push(hr());
        elements.push(process_table("🔥 Top CPU", &output.top_cpu, true));
        elements.push(hr());
        elements.push(process_table("💾 Top Memory", &output.top_memory, false));

        if !output.long_scripts.is_empty() {
            elements.push(hr());
            elements.push(scripts_table(&output.long_scripts));
        }

        if !zombies.is_empty() {
            elements.push(hr());
            elements.push(zombies_table(zombies));
        }

        elements.push(serde_json::json!({
            "tag": "note",
            "elements": [{ "tag": "plain_text", "content": format!("🕐 {timestamp} | {hostname}") }]
        }));

        serde_json::json!({
            "msg_type": "interactive",
            "card": {
                "config": { "wide_screen_mode": true },
                "header": {
                    "title": { "tag": "plain_text", "content": format!("🖥 System Monitor — {hostname}") },
                    "template": header_template(&output.summary)
                },
                "elements": elements
            }
        })
    }
}

fn process_table(
    title: &str,
    processes: &[crate::models::ProcessInfo],
    is_cpu: bool,
) -> serde_json::Value {
    let mut lines = vec![format!("**{title}**")];

    if is_cpu {
        lines.push("PID | %CPU | MEM | NAME".to_string());
    } else {
        lines.push("PID | MEM | NAME".to_string());
    }

    for p in processes {
        let mem = MemoryAnalyzer::format_bytes(p.rss_bytes);
        let name = basename(&p.command);
        if is_cpu {
            lines.push(format!("{} | {:.1}% | {} | {}", p.pid, p.cpu_percent, mem, name));
        } else {
            lines.push(format!("{} | {} | {}", p.pid, mem, name));
        }
    }

    serde_json::json!({
        "tag": "div",
        "text": { "tag": "lark_md", "content": lines.join("\n") }
    })
}

fn scripts_table(scripts: &[crate::models::ProcessInfo]) -> serde_json::Value {
    let mut lines = vec!["**⏳ Long-Running Scripts (> 12h)**".to_string()];
    lines.push("PID | ELAPSED | MEM | NAME".to_string());

    for p in scripts {
        let elapsed = fmt_elapsed(p.elapsed_secs);
        let mem = MemoryAnalyzer::format_bytes(p.rss_bytes);
        let name = basename(&p.command);
        lines.push(format!("{} | {} | {} | {}", p.pid, elapsed, mem, name));
    }

    serde_json::json!({
        "tag": "div",
        "text": { "tag": "lark_md", "content": lines.join("\n") }
    })
}

fn zombies_table(zombies: &[ZombieEntry]) -> serde_json::Value {
    let mut lines = vec!["**⚠️ Zombie Processes**".to_string()];
    lines.push("PID | PPID | PARENT | NAME".to_string());

    for z in zombies {
        let parent = basename(&z.parent_command);
        let name = basename(&z.process.command);
        lines.push(format!("{} | {} | {} | {}", z.process.pid, z.parent_pid, parent, name));
    }

    serde_json::json!({
        "tag": "div",
        "text": { "tag": "lark_md", "content": lines.join("\n") }
    })
}

// --- Helpers ---

fn header_template(summary: &crate::models::SystemSummary) -> &'static str {
    let mem_pct = if summary.total_memory_bytes > 0 {
        summary.used_memory_bytes as f64 / summary.total_memory_bytes as f64 * 100.0
    } else {
        0.0
    };
    if mem_pct > 90.0 {
        "red"
    } else if mem_pct > 70.0 {
        "orange"
    } else {
        "blue"
    }
}

fn summary_section(s: &crate::models::SystemSummary) -> serde_json::Value {
    let mem_used = MemoryAnalyzer::format_bytes(s.used_memory_bytes);
    let mem_total = MemoryAnalyzer::format_bytes(s.total_memory_bytes);
    let mem_pct = if s.total_memory_bytes > 0 {
        s.used_memory_bytes as f64 / s.total_memory_bytes as f64 * 100.0
    } else {
        0.0
    };
    let swap_used = MemoryAnalyzer::format_bytes(s.swap_used_bytes);
    let swap_total = MemoryAnalyzer::format_bytes(s.swap_total_bytes);
    let swap_pct = if s.swap_total_bytes > 0 {
        s.swap_used_bytes as f64 / s.swap_total_bytes as f64 * 100.0
    } else {
        0.0
    };

    let mem_color = threshold_color(mem_pct, 80.0, 60.0);
    let swap_color = threshold_color(swap_pct, 80.0, 60.0);

    let cpu_color = threshold_color(s.cpu_usage_percent, 80.0, 60.0);

    let content = format!(
        "**📊 System Summary**\n\
         CPU:    <font color='{cpu_color}'>{:.1}%</font>\n\
         Memory: <font color='{mem_color}'>{mem_used} / {mem_total} ({mem_pct:.1}%)</font>\n\
         Swap:   <font color='{swap_color}'>{swap_used} / {swap_total} ({swap_pct:.1}%)</font>",
        s.cpu_usage_percent
    );

    serde_json::json!({
        "tag": "div",
        "text": { "tag": "lark_md", "content": content }
    })
}

fn hr() -> serde_json::Value {
    serde_json::json!({ "tag": "hr" })
}

fn threshold_color(pct: f64, red: f64, yellow: f64) -> &'static str {
    if pct > red { "red" } else if pct > yellow { "orange" } else { "green" }
}

fn fmt_elapsed(secs: u64) -> String {
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

fn basename(cmd: &str) -> String {
    let name = cmd.rsplit_once('/').map(|(_, name)| name).unwrap_or(cmd);
    if name.len() > 70 {
        format!("{}...", &name[..67])
    } else {
        name.to_string()
    }
}

