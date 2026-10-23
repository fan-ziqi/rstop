# Changelog

## 0.1.4

- Fix macOS launchd service status, plist discovery and system-domain service inspection.
- Correct macOS APFS storage entries, GPU names and platform probes.
- Improve macOS sampling performance and connection ownership, queue sizes and socket identity.
- Animate loading indicators and limit macOS service logs while preserving native log ordering on other platforms.

## 0.1.3

- Faster resource loading, background process sampling and clearer detail-pane controls.
- Clickable sampling controls, mirrored CPU history, per-device GPU views and searchable Device info.
- Text selection and clipboard copying across pages and popups, with stable selected text and scrollbars that reach the final row.
- An update menu, improved process shortcuts and popup interactions, and double-Escape quit controls.

## 0.1.2

- English update notifications, progress, completion messages and action buttons.
- Project website linked from the package metadata and README.

## 0.1.1

- Quiet background update checks with a compact Release link dialog: upgrade by default, ignore for this session, or persistently skip one version. Checks do not cache release results or block monitoring; read-only/replay sessions remain offline.
- Shared receipt-bound update handling for the TUI and CLI, with network deadlines and explicit version selection.
- Native installer, architecture, checksum, CLI and interactive terminal acceptance on all six release platforms before publication, followed by public-download and previous-version upgrade checks.
- Dependency licenses generated once per release and reused by every archive.

## 0.1.0

- Local CPU, memory, GPU, process, storage, network, container and service monitoring in one Rust TUI for Linux, macOS and Windows.
- Keyboard-labelled controls, inline filters, sortable tables, stable process selection, trees, program grouping and 90 selectable process columns.
- CPU/per-core, memory, network and GPU histories with elapsed-time windows, zoom, peak-preserving curves and nine themes.
- NVIDIA driver/CUDA/NVML information, per-process GPU resources and supported engine, power, clock, PCIe, MIG, NVLink, ECC and throttle fields.
- Local Docker/Podman and service discovery, details, logs and confirmed lifecycle controls; process output inspection, files, environments, scheduling and diagnostic tools.
- Connection ownership and TCP counters, explicit endpoint checks, bounded packet capture, storage investigation and Linux ROS inspection tools.
- Lazy activation with continuous background history after first use and native Linux/Unix collectors.
- Bounded recording/replay, alerts, JSON/CSV/SVG exports, one executable with subcommands, shell completions, a manual and installer-managed updates.
- Prebuilt Linux/macOS/Windows archives with checksums, dependency license notices and GitHub build attestations.

See [COVERAGE.md](COVERAGE.md) for platform/backend limitations and remaining reference-product gaps.
