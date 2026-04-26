use system_monitor::feishu_renderer::FeishuRenderer;
use system_monitor::models::{MonitorOutput, ProcessInfo, ProcessState, SystemSummary};
use system_monitor::zombie_detector::ZombieEntry;

fn make_process(pid: i32, cpu: f64, rss: u64, state: ProcessState) -> ProcessInfo {
    ProcessInfo {
        pid,
        ppid: 1,
        uid: 501,
        user: "testuser".to_string(),
        command: format!("cmd_{pid}"),
        cpu_percent: cpu,
        rss_bytes: rss,
        state,
        elapsed_secs: 0,
    }
}

fn make_output() -> MonitorOutput {
    MonitorOutput {
        summary: SystemSummary {
            cpu_usage_percent: 45.0,
            total_memory_bytes: 32 * 1_073_741_824,
            used_memory_bytes: 16 * 1_073_741_824,
            swap_total_bytes: 5 * 1_073_741_824,
            swap_used_bytes: 1 * 1_073_741_824,
        },
        top_cpu: vec![
            make_process(100, 80.0, 100 * 1024 * 1024, ProcessState::Running),
            make_process(200, 40.0, 200 * 1024 * 1024, ProcessState::Sleeping),
        ],
        top_memory: vec![
            make_process(300, 5.0, 500 * 1024 * 1024, ProcessState::Sleeping),
            make_process(400, 2.0, 300 * 1024 * 1024, ProcessState::Sleeping),
        ],
        long_scripts: vec![make_process(500, 1.0, 50 * 1024 * 1024, ProcessState::Sleeping)],
        zombies: vec![make_process(600, 0.0, 0, ProcessState::Zombie)],
    }
}

// --- FeishuRenderer ---

#[test]
fn feishu_message_is_valid_json() {
    let output = make_output();
    let zombies = vec![ZombieEntry {
        process: make_process(600, 0.0, 0, ProcessState::Zombie),
        parent_pid: 1,
        parent_command: "launchd".to_string(),
    }];
    let msg = FeishuRenderer::build_message(&output, &zombies, "testhost", "2026-04-26 12:00:00");
    assert!(msg.is_object());
    assert_eq!(msg["msg_type"], "interactive");
}

#[test]
fn feishu_card_has_header() {
    let output = make_output();
    let msg = FeishuRenderer::build_message(&output, &[], "my-mac", "2026-04-26");
    let card = &msg["card"];
    assert!(card["header"]["title"]["content"].as_str().unwrap().contains("my-mac"));
}

#[test]
fn feishu_card_has_elements() {
    let output = make_output();
    let zombies = vec![ZombieEntry {
        process: make_process(600, 0.0, 0, ProcessState::Zombie),
        parent_pid: 1,
        parent_command: "launchd".to_string(),
    }];
    let msg = FeishuRenderer::build_message(&output, &zombies, "h", "t");
    let elements = msg["card"]["elements"].as_array().unwrap();
    // At least: summary, hr, cpu, hr, memory, hr, scripts, hr, zombies, hr, note
    assert!(elements.len() >= 8);
}

#[test]
fn feishu_card_has_footer_timestamp() {
    let output = make_output();
    let msg = FeishuRenderer::build_message(&output, &[], "h", "2026-04-26 12:00:00");
    let elements = msg["card"]["elements"].as_array().unwrap();
    let note = elements.last().unwrap();
    let content = note["elements"][0]["content"].as_str().unwrap();
    assert!(content.contains("2026-04-26 12:00:00"));
    assert!(content.contains("h"));
}

#[test]
fn feishu_card_header_color_red_for_high_memory() {
    let mut output = make_output();
    output.summary.used_memory_bytes = 30 * 1_073_741_824; // 93.75%
    let msg = FeishuRenderer::build_message(&output, &[], "h", "t");
    assert_eq!(msg["card"]["header"]["template"], "red");
}

#[test]
fn feishu_card_header_color_blue_for_normal() {
    let output = make_output(); // 50% memory usage
    let msg = FeishuRenderer::build_message(&output, &[], "h", "t");
    assert_eq!(msg["card"]["header"]["template"], "blue");
}

#[test]
fn feishu_card_no_zombies_section_when_empty() {
    let mut output = make_output();
    output.zombies.clear();
    let msg = FeishuRenderer::build_message(&output, &[], "h", "t");
    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(!json_str.contains("Zombie Processes"));
}

#[test]
fn feishu_card_no_scripts_section_when_empty() {
    let mut output = make_output();
    output.long_scripts.clear();
    let msg = FeishuRenderer::build_message(&output, &[], "h", "t");
    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(!json_str.contains("Long-Running Scripts"));
}

#[test]
fn feishu_card_contains_cpu_data() {
    let output = make_output();
    let msg = FeishuRenderer::build_message(&output, &[], "h", "t");
    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("80.0"));
    assert!(json_str.contains("cmd_100"));
}

#[test]
fn feishu_card_contains_memory_data() {
    let output = make_output();
    let msg = FeishuRenderer::build_message(&output, &[], "h", "t");
    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("cmd_300"));
}
