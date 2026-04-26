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

// --- Table builders using Feishu card table element ---

fn process_table(
    title: &str,
    processes: &[crate::models::ProcessInfo],
    is_cpu: bool,
) -> serde_json::Value {
    let mut columns = vec![
        col("pid", "PID"),
        col("user", "USER"),
    ];
    if is_cpu {
        columns.push(col("cpu", "%CPU"));
    }
    columns.push(col("mem", "MEM"));
    columns.push(col("name", "NAME"));

    let rows: Vec<serde_json::Value> = processes
        .iter()
        .map(|p| {
            let mut row = serde_json::json!({
                "pid": p.pid.to_string(),
                "user": p.user,
                "mem": MemoryAnalyzer::format_bytes(p.rss_bytes),
                "name": basename(&p.command).to_string(),
            });
            if is_cpu {
                row["cpu"] = serde_json::json!(format!("{:.1}", p.cpu_percent));
            }
            row
        })
        .collect();

    build_table(title, &columns, &rows)
}

fn scripts_table(scripts: &[crate::models::ProcessInfo]) -> serde_json::Value {
    let columns = vec![
        col("pid", "PID"),
        col("user", "USER"),
        col("elapsed", "ELAPSED"),
        col("mem", "MEM"),
        col("name", "NAME"),
    ];

    let rows: Vec<serde_json::Value> = scripts
        .iter()
        .map(|p| {
            serde_json::json!({
                "pid": p.pid.to_string(),
                "user": p.user,
                "elapsed": fmt_elapsed(p.elapsed_secs),
                "mem": MemoryAnalyzer::format_bytes(p.rss_bytes),
                "name": basename(&p.command).to_string(),
            })
        })
        .collect();

    build_table("⏳ Long-Running Scripts (> 12h)", &columns, &rows)
}

fn zombies_table(zombies: &[ZombieEntry]) -> serde_json::Value {
    let columns = vec![
        col("pid", "PID"),
        col("user", "USER"),
        col("ppid", "PPID"),
        col("parent", "PARENT"),
        col("name", "NAME"),
    ];

    let rows: Vec<serde_json::Value> = zombies
        .iter()
        .map(|z| {
            serde_json::json!({
                "pid": z.process.pid.to_string(),
                "user": z.process.user,
                "ppid": z.parent_pid.to_string(),
                "parent": basename(&z.parent_command).to_string(),
                "name": basename(&z.process.command).to_string(),
            })
        })
        .collect();

    build_table("⚠️ Zombie Processes", &columns, &rows)
}

fn build_table(
    title: &str,
    columns: &[serde_json::Value],
    rows: &[serde_json::Value],
) -> serde_json::Value {
    serde_json::json!({
        "tag": "table",
        "page_size": rows.len().min(10),
        "row_height": "low",
        "header": {
            "title": { "tag": "plain_text", "content": title },
            "template": "blue",
            "ud_icon": { "tag": "standard_icon", "token": "myai-data_outlined" }
        },
        "columns": columns,
        "rows": rows
    })
}

fn col(name: &str, display: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "display_name": { "tag": "plain_text", "content": display },
        "data_type": "text",
        "width": "auto"
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

fn basename(cmd: &str) -> &str {
    cmd.rsplit_once('/').map(|(_, name)| name).unwrap_or(cmd)
}
