# ratatui-guardian

> Render Budget Guardian — frame budget tracking and waste detection for ratatui TUI apps.

[![CI](https://github.com/SuperInstance/ratatui/actions/workflows/ci.yml/badge.svg?branch=guardian)](https://github.com/SuperInstance/ratatui/actions/workflows/ci.yml)

Most TUI apps silently waste render time: full-screen redraws for a 2-cell change, widgets that allocate `String` on every frame, layouts nested 5+ deep. **Guardian makes that waste visible.**

## Quick Start

```rust
use ratatui_guardian::{FrameBudget, RenderProfiler};

let budget = FrameBudget::for_60fps();
let mut profiler = RenderProfiler::new(budget);

profiler.begin_frame();
profiler.begin_widget("StatusBar");
// ... your widget renders ...
profiler.end_widget(80); // cells written
profiler.end_frame();

println!("{}", profiler.report());
```

## Features

| Feature | Description |
|---------|-------------|
| **Frame budget tracking** | Detect frames that exceed time, cell, or depth limits |
| **Waste detection** | Identify hogs, full-redraw waste, suspected allocations, deep nesting |
| **Automatic cell tracking** | `GuardianBuffer` adapter intercepts cell writes |
| **Persistence** | Save/load profiler state as JSON for historical analysis |
| **Multi-format export** | JSON, Prometheus, CSV for dashboards and monitoring |
| **Trend analysis** | Compare two profiler snapshots to detect degradation |

## API Reference

### `FrameBudget`

Defines performance constraints for a render frame.

```rust
// Presets
let budget = FrameBudget::for_60fps();  // 16ms per frame
let budget = FrameBudget::for_30fps();  // 33ms per frame

// Custom
let budget = FrameBudget::new(
    Duration::from_millis(20),  // max render time
    5000,                        // max diff cells
    4,                           // max widget depth
);
```

### `RenderProfiler`

The core profiler. Wraps your render loop to track per-widget timing and cells.

| Method | Description |
|--------|-------------|
| `new(budget)` | Create a profiler with the given budget |
| `begin_frame()` | Start timing a new frame |
| `begin_widget(name)` | Start timing a widget |
| `end_widget(cells)` | End timing, report cells written |
| `end_frame()` | End frame, run waste detection |
| `report()` | Get human-readable `ReportFormatter` |
| `save_json(path)` | Persist state to JSON file |
| `load_json(path, budget)` | Restore state from JSON file |
| `compare(other, threshold)` | Trend analysis against another profiler |
| `reset()` | Clear all accumulated state |

### `GuardianBuffer` / `GuardianFrame`

Automatic cell tracking without manual counting:

```rust
use ratatui_guardian::ratatui_adapter::GuardianFrame;

let mut frame = GuardianFrame::begin();
let mut buf = frame.track_widget("Header");
// Your widget calls buf.set_cell() or buf.set_string(len)
frame.finish_widget(buf);
let report = frame.end();
```

### `Reporter`

Multi-format export:

```rust
use ratatui_guardian::Reporter;

let reporter = Reporter::from_profiler(&profiler);
println!("{}", reporter.to_json());       // Structured JSON
println!("{}", reporter.to_prometheus()); // Prometheus exposition format
println!("{}", reporter.to_csv());        // CSV for spreadsheets
```

### `GuardianError`

All fallible operations return `Result<T, GuardianError>`:

- `Io` — file read/write errors
- `Json` — serialization/deserialization errors
- `NoData` — operation requires data that doesn't exist
- `InvalidConfig` — bad parameters
- `ComparisonFailed` — incompatible profiler states

### `TrendReport`

Result of comparing two profiler states:

```rust
let trend = current.compare(&baseline, 0.25)?;
for d in &trend.degraded {
    println!("↗ {}: +{:.1}%", d.name, d.change_percent);
}
if trend.significant_degradation {
    eprintln!("Performance regression detected!");
}
```

## Waste Detection Heuristics

| Finding | Severity | Trigger |
|---------|----------|---------|
| **Hog** | Warning/Critical | Widget consumes >60% of frame time |
| **Full Redraw** | Hint/Warning | Widget redraws all cells when few changed |
| **Suspected Allocation** | Hint/Warning | Render time suggests per-frame allocations |
| **Deep Nesting** | Hint | Layout nesting exceeds depth limit |

All thresholds are configurable via `DetectionConfig`.

## Examples

```bash
# Minimal — 10-line instrumentation
cargo run --example minimal

# Full demo — all features
cargo run --example full_demo
```

## License

MIT
