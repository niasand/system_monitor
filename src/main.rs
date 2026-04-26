use std::time::Duration;

use clap::Parser;
use system_monitor::collector::ProcessCollector;
use system_monitor::cpu_analyzer::CpuAnalyzer;
use system_monitor::memory_analyzer::MemoryAnalyzer;
use system_monitor::models::MonitorOutput;
use system_monitor::renderer::Renderer;
use system_monitor::script_detector::ScriptDetector;
use system_monitor::zombie_detector::ZombieDetector;
use system_monitor::collector;

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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let current_uid = unsafe { libc::getuid() };

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

        let renderer = Renderer::new(!args.no_color, args.json);
        let rendered = renderer.render(&output, &zombie_entries);
        println!("{rendered}");

        if args.watch.is_none() {
            std::process::exit(if has_zombies { 1 } else { 0 });
        }

        if !args.json {
            print!("\x1b[2J\x1b[H");
        }
        std::thread::sleep(Duration::from_secs(args.watch.unwrap()));
    }
}
