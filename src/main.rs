use std::time::{Duration, Instant};

use chrono::Local;
use clap::Parser;
use system_monitor::collector;
use system_monitor::collector::ProcessCollector;
use system_monitor::cpu_analyzer::CpuAnalyzer;
use system_monitor::feishu_notifier::FeishuNotifier;
use system_monitor::feishu_renderer::FeishuRenderer;
use system_monitor::memory_analyzer::MemoryAnalyzer;
use system_monitor::models::MonitorOutput;
use system_monitor::renderer::Renderer;
use system_monitor::script_detector::ScriptDetector;
use system_monitor::zombie_detector::ZombieDetector;

#[derive(Parser)]
#[command(name = "system_monitor", about = "macOS system resource monitor")]
struct Args {
    /// Number of top processes to show
    #[arg(short, long, default_value = "10")]
    top: usize,

    /// Auto-refresh interval in seconds
    #[arg(short, long)]
    watch: Option<u64>,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Script runtime threshold in hours (default: 12)
    #[arg(long)]
    threshold: Option<u64>,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Feishu bot webhook URL (also reads FEISHU_WEBHOOK_URL env var)
    #[arg(long)]
    feishu_webhook: Option<String>,

    /// Feishu push interval in seconds (default: 1800)
    #[arg(long, default_value = "1800")]
    feishu_interval: u64,
}

fn resolve_feishu_url(cli_value: Option<String>) -> Option<String> {
    cli_value.or_else(|| std::env::var("FEISHU_WEBHOOK_URL").ok())
}

fn get_hostname() -> String {
    let mut buf = [0u8; 256];
    unsafe {
        if libc::gethostname(buf.as_mut_ptr() as *mut i8, buf.len()) == 0 {
            let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            return String::from_utf8_lossy(&buf[..len]).to_string();
        }
    }
    "unknown".to_string()
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let current_uid = unsafe { libc::getuid() };
    let feishu_url = resolve_feishu_url(args.feishu_webhook.clone());

    if feishu_url.is_some() && args.watch.is_none() {
        eprintln!("Error: --feishu-webhook requires --watch mode for continuous monitoring");
        std::process::exit(2);
    }

    let feishu = feishu_url.map(FeishuNotifier::new);
    let hostname = get_hostname();
    let mut last_push = Instant::now() - Duration::from_secs(args.feishu_interval);

    loop {
        let processes = ProcessCollector::new().collect()?;

        let top_cpu = CpuAnalyzer::new(args.top).analyze(&processes);
        let top_memory = MemoryAnalyzer::new(args.top).analyze(&processes);
        let long_scripts = ScriptDetector::new(current_uid)
            .with_threshold(args.threshold.unwrap_or(12) * 3600)
            .detect(&processes);
        let zombie_entries = ZombieDetector::new().detect(&processes);
        let summary = collector::get_system_summary()?;

        let has_zombies = !zombie_entries.is_empty();
        let output = MonitorOutput {
            summary,
            top_cpu,
            top_memory,
            long_scripts,
            zombies: zombie_entries.iter().map(|z| z.process.clone()).collect(),
        };

        // Terminal output
        let renderer = Renderer::new(!args.no_color, args.json);
        let rendered = renderer.render(&output, &zombie_entries);
        println!("{rendered}");

        // Feishu push
        if let Some(ref notifier) = feishu {
            if last_push.elapsed() >= Duration::from_secs(args.feishu_interval) {
                let ts = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let msg = FeishuRenderer::build_message(&output, &zombie_entries, &hostname, &ts);
                if let Err(e) = notifier.send(&msg) {
                    eprintln!("[feishu] {e}");
                }
                last_push = Instant::now();
            }
        }

        if args.watch.is_none() {
            std::process::exit(if has_zombies { 1 } else { 0 });
        }

        if !args.json {
            print!("\x1b[2J\x1b[H");
        }
        std::thread::sleep(Duration::from_secs(args.watch.unwrap()));
    }
}
