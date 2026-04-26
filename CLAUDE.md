# system_monitor — macOS System Monitor CLI

## Project Overview

A lightweight Rust CLI tool that surfaces four monitoring panels:
- Top N CPU-consuming processes
- Top N memory-consuming processes
- User-owned scripts running > 12 hours
- Zombie processes with parent info

Target: macOS only (v1). Single binary, zero runtime deps.

## Tech Stack

- **Language**: Rust (edition 2024)
- **CLI**: clap (derive)
- **Color output**: colored / ANSI
- **System calls**: nix + libc (proc_listpids, proc_pidinfo, sysctl)
- **Serialization**: serde + serde_json

## Build & Run

```bash
cargo build --release
./target/release/system_monitor
./target/release/system_monitor --watch 5
./target/release/system_monitor --json
```

## Architecture

```
src/
  main.rs              # CLI entrypoint, clap args
  collector.rs         # ProcessCollector — syscall-based process gathering
  models.rs            # ProcessInfo struct, shared types
  cpu_analyzer.rs      # Top N CPU sorting
  memory_analyzer.rs   # Top N memory sorting
  script_detector.rs   # Long-running script detection
  zombie_detector.rs   # Zombie process detection
  renderer.rs          # Terminal table + JSON output
```

Data flow:
```
CLI Args → ProcessCollector → [CpuAnalyzer, MemoryAnalyzer, ScriptDetector, ZombieDetector] → Renderer → stdout
```

## Coding Conventions

- Follow `cargo clippy` with `-D warnings` — zero warnings allowed
- Format with `cargo fmt` before every commit
- Use `#[derive(Debug, Clone)]` on all data structs
- Error handling: use `anyhow::Result` in CLI layer, custom enum errors in library modules
- No `unwrap()` in library code — only in tests
- Keep modules pure: analyzers take `&[ProcessInfo]` and return computed results, no side effects
- Public API through clearly defined traits where useful, but don't over-abstract for single implementations

## Testing Strategy

- **Unit tests**: analyzers (CpuAnalyzer, MemoryAnalyzer, ScriptDetector, ZombieDetector) — pure functions, easy to test with mock ProcessInfo fixtures
- **Integration tests**: ProcessCollector against real macOS syscalls
- **Snapshot tests**: Renderer output formatting
- **E2E**: binary runs and exits with correct codes

```bash
cargo test                    # all tests
cargo test --test integration # integration only
```

## Exit Codes

- `0`: Normal run, no zombie processes found
- `1`: Zombie processes detected
- `2`: Collection error (permissions, syscall failure)

## Commit Style

English commit messages, format: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`.

## Out of Scope (v1)

- GUI, remote monitoring, process manipulation (kill/renice)
- History/persistence, Linux support, Docker, GPU, disk I/O
- Config files (TOML/YAML)
