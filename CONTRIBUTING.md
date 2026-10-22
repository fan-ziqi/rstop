# Contributing

Start with [Run from source](README.md#run-from-source) for prerequisites, cloning, and development/release launch commands. Use the toolchain in [rust-toolchain.toml](rust-toolchain.toml). Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked` before submitting a change.

Use one executable with subcommands: `rstop <command>`. Keep user-facing documentation in English. When CLI arguments change, regenerate the manual/completions with `python3 scripts/distribution.py docs --binary target/debug/rstop` after building.

Keep unknown, unsupported, and permission-denied telemetry distinct. Tests must use owned processes/files. Live systemd discovery, transient-service control and sourced ROS tests are explicitly opt-in; a generic CI runner is not a guaranteed service/ROS host. Cross-compilation and generic CI checks are not real-device feature acceptance. Record platform/hardware limits in [COVERAGE.md](COVERAGE.md), and performance methodology/results in [BENCHMARK.md](BENCHMARK.md).

See [RELEASING.md](RELEASING.md) for distribution. Contributions are provided under Apache-2.0 unless an explicitly documented third-party license applies. Retain provenance and notices for any reused code; reference-project acknowledgement alone is insufficient.

Collectors own OS/driver reads and hand off `model::Snapshot` data; `history` owns bounded time series, `App` owns interaction state, and `ui` owns rendering and matching hit regions. Runtime workers coordinate fast sampling, slow discovery and explicit inspections. Reuse these boundaries and shared table/chart controls rather than adding per-page copies.

Public documentation has one role per file: README for use, COVERAGE for supported fields and limitations, BENCHMARK for measured performance, CHANGELOG for release changes, and RELEASING for maintainers. Keep local logs/host snapshots under ignored `artifacts/`; only publish deliberate, non-sensitive fixtures and measurement evidence.
