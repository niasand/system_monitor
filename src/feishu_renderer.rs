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
        elements.push(process_section(
            "🔥 Top CPU",
            &output.top_cpu,
            true,
        ));
        elements.push(hr());
        elements.push(process_section(
            "💾 Top Memory",
            &output.top_memory,
            false,
        ));

        if !output.long_scripts.is_empty() {
            elements.push(hr());
            elements.push(scripts_section(&output.long_scripts));
        }

        if !zombies.is_empty() {
            elements.push(hr());
            elements.push(zombies_section(zombies));
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

    let content = format!(
        "**📊 System Summary**\n\
         Memory: <font color='{mem_color}'>{mem_used} / {mem_total} ({mem_pct:.1}%)</font>\n\
         Swap:   <font color='{swap_color}'>{swap_used} / {swap_total} ({swap_pct:.1}%)</font>"
    );

    serde_json::json!({
        "tag": "div",
        "text": { "tag": "lark_md", "content": content }
    })
}

fn process_section(
    title: &str,
    processes: &[crate::models::ProcessInfo],
    is_cpu: bool,
) -> serde_json::Value {
    let mut lines = vec![format!("**{title}**")];
    lines.push("```".to_string());
    lines.push(format!(
        "{:<7} {:<10} {:>6} {:>10}  {}",
        "PID", "USER", if is_cpu { "%CPU" } else { "%MEM" }, "MEM", "COMMAND"
    ));

    for p in processes {
        let mem = MemoryAnalyzer::format_bytes(p.rss_bytes);
        let cmd = truncate_str(&p.command, 25);
        lines.push(format!(
            "{:<7} {:<10} {:>6.1} {:>10}  {}",
            p.pid, p.user, p.cpu_percent, mem, cmd
        ));
    }
    lines.push("```".to_string());

    serde_json::json!({
        "tag": "div",
        "text": { "tag": "lark_md", "content": lines.join("\n") }
    })
}

fn scripts_section(scripts: &[crate::models::ProcessInfo]) -> serde_json::Value {
    let mut lines = vec!["**⏳ Long-Running Scripts (> 12h)**".to_string()];
    lines.push("```".to_string());
    lines.push(format!(
        "{:<7} {:<10} {:>12} {:>10}  {}",
        "PID", "USER", "ELAPSED", "MEM", "COMMAND"
    ));

    for p in scripts {
        let elapsed = fmt_elapsed(p.elapsed_secs);
        let mem = MemoryAnalyzer::format_bytes(p.rss_bytes);
        let cmd = truncate_str(&p.command, 25);
        lines.push(format!(
            "{:<7} {:<10} {:>12} {:>10}  {}",
            p.pid, p.user, elapsed, mem, cmd
        ));
    }
    lines.push("```".to_string());

    serde_json::json!({
        "tag": "div",
        "text": { "tag": "lark_md", "content": lines.join("\n") }
    })
}

fn zombies_section(zombies: &[ZombieEntry]) -> serde_json::Value {
    let mut lines = vec!["**⚠️ Zombie Processes**".to_string()];
    lines.push("```".to_string());
    lines.push(format!(
        "{:<7} {:<10} {:>7} {:<15} {}",
        "PID", "USER", "PPID", "PARENT", "COMMAND"
    ));

    for z in zombies {
        let parent = truncate_str(&z.parent_command, 15);
        let cmd = truncate_str(&z.process.command, 20);
        lines.push(format!(
            "{:<7} {:<10} {:>7} {:<15} {}",
            z.process.pid, z.process.user, z.parent_pid, parent, cmd
        ));
    }
    lines.push("```".to_string());

    serde_json::json!({
        "tag": "div",
        "text": { "tag": "lark_md", "content": lines.join("\n") }
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

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max - 3]) }
}
