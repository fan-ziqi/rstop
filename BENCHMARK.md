# Performance comparison

Measured on one Linux host on 2026-10-22: three rounds per mode, six seconds of warm-up, twelve seconds of measurement, a 160×48 PTY, and one-second refresh intervals. CPU uses 100% per logical core. Tables show medians; ranges show the three CPU readings.

The [first batch](benchmarks/2026-10-22-performance-linux.json) interleaves the frozen pre-optimization rstop with btop, nvitop, top and a preliminary optimized build. The [final batch](benchmarks/2026-10-22-performance-final-linux.json) measures the installed rstop after an unknown-count display fix and first-sample counter reset fix. Each batch runs sequentially in randomized order. The table uses final-batch rstop results and first-batch competitors/baseline; these are separate time windows, not one interleaved trial. Raw reports retain commands, hashes, samples, load and versions. All 36 runs completed.

| Mode | Main CPU % | CPU incl. helpers % (range) | Main RSS MiB | Tree PSS MiB | TTY KiB/s |
|---|---:|---:|---:|---:|---:|
| rstop before optimization, eager overview | 4.41 | 8.31 (8.30–8.72) | 41.59 | 39.72 | 1.22 |
| rstop final, unopened Overview | 0.59 | 0.59 (0.42–0.59) | 29.25 | 26.08 | 1.15 |
| rstop final, Processes | 2.12 | 2.12 (2.03–2.12) | 35.25 | 32.48 | 5.78 |
| rstop final, GPU | 2.21 | 2.21 (2.03–2.21) | 35.50 | 32.55 | 1.41 |
| rstop final, all sources enabled | 4.24 | 4.41 (4.41–4.50) | 38.74 | 36.56 | 1.25 |
| btop 1.4.7, static / no GPU | 0.68 | 0.68 (0.68–0.68) | 3.25 | 3.54 | 29.15 |
| nvitop 1.7.1 | 0.84 | 0.84 (0.76–0.93) | 41.50 | 34.94 | 0.30 |
| top / procps-ng 3.3.17 | 0.76 | 0.76 (0.59–0.76) | 4.25 | 1.77 | 7.73 |

Final default tree CPU is 92.8% lower than the frozen eager baseline; with every source enabled it is 46.9% lower. The default reduction combines lazy activation with collector optimizations. It does not isolate implementation speed. Full-source sampling is the closer scope comparison, though transient workloads and data can still differ.

Default Overview samples host CPU/RAM/storage/interfaces and physical GPU counters. Processes/GPU activate process metadata and per-process GPU attribution. Container, service and detailed connection sources start on first use and stay active for the rest of the session. `--full-sampling` activates all of these immediately; it represents the background sources after visiting all pages. Slow discovery waits five seconds between scans. Optional probes, extra columns, active logs, recording and process-network tracing were disabled.

btop's measured [official 1.4.7 static build](https://github.com/aristocratos/btop/releases/tag/v1.4.7) reports `GPU_SUPPORT=false`. nvitop targets NVIDIA GPU processes; top targets host processes. Their scopes differ from rstop. Full-source rstop still uses more CPU than these tools; these short Linux measurements do not establish a universal speed ranking or performance on macOS/Windows.

## Implementation and validation

Linux process sampling reuses bounded procfs descriptors, socket counters use local socket diagnostics, and services use read-only systemd D-Bus. Unix Docker can read its local Engine socket. Existing collectors/CLI fallbacks remain available. The pipeline annotates each sample once; NVIDIA capability metadata is cached while supported dynamic counters keep updating.

Measured and installed final binary SHA-256: `a45479ec22586f0a699143f0de212673a65e3d895a8256256704d58e1bcf7278`. Frozen before binary: `6f112e85d089f586547351e4b0ed0a3f5c3e347d0acfc7333394e9bdbc8bf906`. The preliminary artifact in the first batch is identified separately in its metadata and is not used for final rstop rows.

[Validation evidence](benchmarks/2026-10-22-performance-validation.json): 182 Rust tests passed, two opt-in tests ignored; Clippy passed with warnings denied; 198 PTY checks, nine lazy-source checks, public CLI and generated manual/completions passed. A hidden-chart regression preserves every GPU sample and timestamp while another page is active. Connect-only tracing confirmed unopened Docker/systemd sources made zero connection attempts and kept making new attempts after leaving their pages. Connection attempts do not prove successful API reads.

The Docker daemon was stopped during this batch. Live container-stat parity and four container-management PTY checks were skipped without starting a daemon. An owned mock Engine Unix socket verifies HTTP discovery, running/stopped containers, CPU/memory units and IPv6 ports. Real systemd reads, local TCP transfer tests and GPU version fields were checked separately. Active-log, GPU-stress and sustained multi-hour overhead were not benchmarked.

## Earlier multi-tool baseline

These measurements used an older rstop build, SHA-256 `9003eb9c2d7e630143b62ca52bdd402d8b89baa2e2697d195791931b0bc42234`. They do not describe the current optimized build. [30 raw runs](benchmarks/2026-10-22-linux.json) retain exact commands, versions, resources and conditions. The final two runs resumed sequentially after harness termination; segment times/load remain recorded.

| Tool / mode | Version | Main CPU % | CPU including helpers % | Main RSS MiB | Process-tree PSS MiB | TTY KiB/s |
|---|---|---:|---:|---:|---:|---:|
| rstop default | 0.2.0 | 4.51 | 5.94 | 40.48 | 40.17 | 2.16 |
| rstop GPU | 0.2.0 | 4.21 | 5.64 | 36.38 | 36.02 | 0.80 |
| rstop all columns | 0.2.0 | 10.14 | 11.68 | 75.25 | 74.31 | 2.42 |
| btop | 1.2.3 | 1.01 | 1.01 | 6.25 | 3.05 | 29.44 |
| htop | 3.0.5 | 4.00 | 4.00 | 12.00 | 10.04 | 4.17 |
| bottom | 0.14.9 | 2.97 | 2.97 | 27.75 | 26.67 | 1.30 |
| glances | 4.5.8 | 6.13 | 6.13 | 81.62 | 82.39 | 3.36 |
| nvitop | 1.7.1 | 0.91 | 0.91 | 40.72 | 36.75 | 0.25 |
| nvtop | 1.2.2 | 0.91 | 0.91 | 24.25 | 22.52 | 0.28 |
| top | procps 3.3.17 | 0.71 | 0.71 | 4.50 | 1.94 | 8.22 |


Other retained batches: [compact layout](benchmarks/2026-10-22-compact-linux.json), [keyboard controls](benchmarks/2026-10-22-keyboard-linux.json), [process selection before visible-row rendering](benchmarks/2026-10-22-selection-linux.json), [selection and visible rows](benchmarks/2026-10-22-selection-visible-linux.json). Each report identifies its measured revision; background load and feature changes prevent attributing differences to one optimization.

## Host and method

AMD Ryzen 9 9950X, 32 logical CPUs, approximately 60.4 GiB RAM; Ubuntu 22.04 / Linux 6.8.0-138. NVIDIA RTX 4090 and AMD integrated GPU, readable local systemd units. The Docker daemon was stopped. Existing desktop/background workloads continued running. Host load and executable hashes are retained per batch.

TERM=xterm-256color, COLORTERM=truecolor, NO_COLOR unset; isolated monitor configuration. Main CPU includes all monitor threads. Tree CPU includes live descendants and reaped child CPU, scanning every thread's child list. Work inside existing service/container daemons is not attributed. Thus native IPC may move CPU outside the measured tree. Main RSS is each run's time median; tree PSS is sampled once per second. Summed RSS double-counts shared pages; sampling can miss short-lived helpers. RSS/PSS reads are not simultaneous.

TTY KiB/s counts bytes including control sequences. First output in the JSON includes terminal setup bytes and does not measure full-data readiness. Native counters that are denied or unsupported remain unknown. The cached-descriptor path uses additional bounded kernel file descriptors; their kernel memory is not included in RSS/PSS.

## Reproduce

```sh
cargo build --release --locked
mkdir -p artifacts/tools
cp target/release/rstop artifacts/perf-final-v2
# Restore the preserved before binary and download the official btop 1.4.7 static build
# to the paths in the manifest; nvitop and top must be on PATH.
python3 scripts/benchmark.py --manifest benchmarks/performance-tools.json --output artifacts/benchmark-performance.json --rounds 3 --warmup 6 --duration 12
python3 scripts/check_lazy.py artifacts/perf-final-v2
```

The before artifact and btop binary are preserved locally under `artifacts/`; they are not distributed in the source repository. Executable hashes in the raw reports identify them. To measure only available/current tools, remove unavailable entries from a copy of the manifest. The final four-mode run used that same subset. The harness stops only monitor processes it creates; all monitor runs use read-only mode where available. `check_lazy.py` requires Linux, strace, pyte, Docker CLI and systemd. Developer stage timings are available with `cargo run --release --example profile -- 10`; these eager, parent-only timings are not a TUI/process-tree benchmark.
