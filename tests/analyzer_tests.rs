use system_monitor::cpu_analyzer::CpuAnalyzer;
use system_monitor::memory_analyzer::MemoryAnalyzer;
use system_monitor::models::{ProcessInfo, ProcessState};
use system_monitor::script_detector::ScriptDetector;
use system_monitor::zombie_detector::ZombieDetector;

fn make_process(
    pid: i32,
    ppid: i32,
    uid: u32,
    command: &str,
    cpu: f64,
    rss: u64,
    state: ProcessState,
    elapsed: u64,
) -> ProcessInfo {
    ProcessInfo {
        pid,
        ppid,
        uid,
        user: "testuser".to_string(),
        command: command.to_string(),
        cpu_percent: cpu,
        rss_bytes: rss,
        state,
        elapsed_secs: elapsed,
    }
}

// --- CpuAnalyzer ---

#[test]
fn cpu_analyzer_returns_top_n_sorted() {
    let procs = vec![
        make_process(1, 0, 501, "a", 10.0, 0, ProcessState::Running, 0),
        make_process(2, 0, 501, "b", 90.0, 0, ProcessState::Running, 0),
        make_process(3, 0, 501, "c", 50.0, 0, ProcessState::Running, 0),
        make_process(4, 0, 501, "d", 30.0, 0, ProcessState::Running, 0),
    ];
    let result = CpuAnalyzer::new(2).analyze(&procs);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].pid, 2); // 90%
    assert_eq!(result[1].pid, 3); // 50%
}

#[test]
fn cpu_analyzer_fewer_than_n() {
    let procs = vec![
        make_process(1, 0, 501, "a", 10.0, 0, ProcessState::Running, 0),
    ];
    let result = CpuAnalyzer::new(10).analyze(&procs);
    assert_eq!(result.len(), 1);
}

#[test]
fn cpu_analyzer_empty() {
    let result = CpuAnalyzer::new(5).analyze(&[]);
    assert!(result.is_empty());
}

// --- MemoryAnalyzer ---

#[test]
fn memory_analyzer_returns_top_n_sorted() {
    let procs = vec![
        make_process(1, 0, 501, "a", 0.0, 100, ProcessState::Running, 0),
        make_process(2, 0, 501, "b", 0.0, 500, ProcessState::Running, 0),
        make_process(3, 0, 501, "c", 0.0, 300, ProcessState::Running, 0),
    ];
    let result = MemoryAnalyzer::new(2).analyze(&procs);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].pid, 2); // 500 bytes
    assert_eq!(result[1].pid, 3); // 300 bytes
}

#[test]
fn format_bytes_gb() {
    assert_eq!(MemoryAnalyzer::format_bytes(2_147_483_648), "2.0 GB");
}

#[test]
fn format_bytes_mb() {
    assert_eq!(MemoryAnalyzer::format_bytes(100 * 1_048_576), "100.0 MB");
}

#[test]
fn format_bytes_kb() {
    assert_eq!(MemoryAnalyzer::format_bytes(4096), "4 KB");
}

// --- ScriptDetector ---

#[test]
fn script_detector_filters_by_uid_and_elapsed() {
    let uid = 501;
    let procs = vec![
        make_process(1, 0, uid, "python", 0.0, 0, ProcessState::Running, 13 * 3600),
        make_process(2, 0, uid, "python", 0.0, 0, ProcessState::Running, 6 * 3600),  // too short
        make_process(3, 0, 0, "python", 0.0, 0, ProcessState::Running, 13 * 3600),   // wrong uid
        make_process(4, 0, uid, "chrome", 0.0, 0, ProcessState::Running, 13 * 3600), // not interpreter
    ];
    let result = ScriptDetector::new(uid).detect(&procs);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].pid, 1);
}

#[test]
fn script_detector_custom_threshold() {
    let uid = 501;
    let procs = vec![
        make_process(1, 0, uid, "bash", 0.0, 0, ProcessState::Running, 6 * 3600),
    ];
    let result = ScriptDetector::new(uid).with_threshold(5 * 3600).detect(&procs);
    assert_eq!(result.len(), 1);
}

#[test]
fn script_detector_interpreter_variants() {
    let uid = 501;
    let procs = vec![
        make_process(1, 0, uid, "python3", 0.0, 0, ProcessState::Running, 13 * 3600),
        make_process(2, 0, uid, "node", 0.0, 0, ProcessState::Running, 13 * 3600),
        make_process(3, 0, uid, "ruby", 0.0, 0, ProcessState::Running, 13 * 3600),
        make_process(4, 0, uid, "zsh", 0.0, 0, ProcessState::Running, 13 * 3600),
    ];
    let result = ScriptDetector::new(uid).detect(&procs);
    assert_eq!(result.len(), 4);
}

// --- ZombieDetector ---

#[test]
fn zombie_detector_finds_zombies_with_parent() {
    let procs = vec![
        make_process(1, 0, 0, "launchd", 0.0, 0, ProcessState::Running, 0),
        make_process(2, 1, 501, "app", 0.0, 0, ProcessState::Zombie, 0),
        make_process(3, 1, 501, "other", 0.0, 0, ProcessState::Running, 0),
    ];
    let result = ZombieDetector::new().detect(&procs);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].process.pid, 2);
    assert_eq!(result[0].parent_pid, 1);
    assert_eq!(result[0].parent_command, "launchd");
}

#[test]
fn zombie_detector_no_zombies() {
    let procs = vec![
        make_process(1, 0, 0, "a", 0.0, 0, ProcessState::Running, 0),
    ];
    let result = ZombieDetector::new().detect(&procs);
    assert!(result.is_empty());
}

#[test]
fn zombie_detector_parent_not_found() {
    let procs = vec![
        make_process(1, 9999, 501, "orphan_zombie", 0.0, 0, ProcessState::Zombie, 0),
    ];
    let result = ZombieDetector::new().detect(&procs);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].parent_command, ""); // parent gone
}
