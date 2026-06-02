# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-06-02

### Added

- **Data source adapter** — `GuardianBuffer` and `GuardianFrame` for automatic cell tracking without manual instrumentation
- **Persistence** — `Profiler::save_json(path)` and `Profiler::load_json(path, budget)` for historical frame analysis
- **Export formats** — `Reporter` with `to_json()`, `to_prometheus()`, `to_csv()` for dashboards and monitoring
- **Error handling** — `GuardianError` enum replacing all silent drops and panics
- **Trend analysis** — `Profiler::compare(previous, threshold)` with `TrendReport` for detecting performance degradation
- **CI/CD** — GitHub Actions workflow with cargo test, clippy, and rustfmt on stable
- **Integration examples** — `examples/minimal.rs` and `examples/full_demo.rs`
- **Documentation** — Complete API reference in README, architecture.md, CONTRIBUTING.md

### Fixed (carried from v0.1)

- Division-by-zero guard in hog detection (tiny frame times)
- Unmatched `end_widget` calls now increment a counter instead of silently dropping
- Deep nesting check uses actual tracked depth, not total widget count
- `full_redraw_allowed` flag suppresses the full-redraw heuristic for whitelisted widgets
- Configurable detection thresholds via `DetectionConfig`
- `to_json()` export method on profiler
- `reset()` clears all accumulated state including error counters

## [0.1.0] — 2026-06-02

### Added

- Initial release with core profiling, waste detection, and budget tracking
- Bug fixes for 8 issues found during Module 2 audit
