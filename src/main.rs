mod collector;
mod cpu_analyzer;
mod memory_analyzer;
mod models;
mod renderer;
mod script_detector;
mod zombie_detector;

use std::time::Duration;

use clap::Parser;
use collector::ProcessCollector;
use cpu_analyzer::CpuAnalyzer;
use memory_analyzer::MemoryAnalyzer;
use models::MonitorOutput;
use renderer::Renderer;
use script_detector::ScriptDetector;
use zombie_detector::ZombieDetector;

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
        let output = run_once(&args, current_uid)?;

        let has_zombies = !output.zombies.is_empty();
        let renderer = Renderer::new(!args.no_color, args.json);
        let zombies = ZombieDetector::new().detect(&output.top_cpu); // placeholder
        let rendered = renderer.render(&output, &zombies);
        println!("{rendered}");

        if args.watch.is_none() {
            std::process::exit(if has_zombies { 1 } else { 0 });
        }

        std::thread::sleep(Duration::from_secs(args.watch.unwrap()));
    }
}

fn run_once(args: &Args, current_uid: u32) -> anyhow::Result<MonitorOutput> {
    let processes = ProcessCollector::new().collect()?;

    let top_cpu = CpuAnalyzer::new(args.top).analyze(&processes);
    let top_memory = MemoryAnalyzer::new(args.top).analyze(&processes);

    let threshold_secs = args.threshold.unwrap_or(12) * 3600;
    let long_scripts = ScriptDetector::new(current_uid)
        .with_threshold(threshold_secs)
        .detect(&processes);

    let zombies = ZombieDetector::new().detect(&processes);

    let summary = collector::get_system_summary()?;

    Ok(MonitorOutput {
        summary,
        top_cpu,
        top_memory,
        long_scripts,
        zombies: zombies.into_iter().map(|z| z.process).collect(),
    })
}
