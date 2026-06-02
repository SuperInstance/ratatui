# Architecture

## Module Structure

```
src/
├── lib.rs              # Public API, re-exports
├── budget.rs           # FrameBudget, DetectionConfig, BudgetViolation
├── profiler.rs         # RenderProfiler, PerWidgetStats, persistence, trend analysis
├── detector.rs         # WasteDetector, WasteFinding, WasteCategory
├── report.rs           # ReportFormatter (human-readable terminal output)
├── reporter.rs         # Reporter (JSON, Prometheus, CSV export)
├── ratatui_adapter.rs  # GuardianBuffer, GuardianFrame (auto cell tracking)
└── error.rs            # GuardianError enum
```

## Data Flow

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│ Your App    │────>│ RenderProfiler│────>│ WasteDetector│
│ begin_frame │     │ begin_widget  │     │ detect()     │
│ begin_widget│     │ end_widget    │     │ stash_findings│
│ end_widget  │     │ end_frame     │     └──────┬───────┘
│ end_frame   │     └──────┬───────┘            │
└─────────────┘            │                    │
                           ▼                    ▼
                  ┌────────────────┐    ┌──────────────┐
                  │ ReportFormatter│    │ Reporter     │
                  │ (Display)      │    │ to_json()    │
                  └────────────────┘    │ to_prometheus│
                                        │ to_csv()     │
                                        └──────────────┘
```

## Key Design Decisions

1. **No ratatui dependency** — Guardian is framework-agnostic. The `ratatui_adapter` module provides helpers that work with any buffer type.

2. **Stateless detector** — `WasteDetector` holds only the last frame's findings. All context comes from profiler data passed to `detect()`.

3. **Bounded history** — Frame history defaults to 120 entries (2 seconds at 60fps) with FIFO eviction.

4. **All errors are GuardianError** — No silent drops, no panics on bad input. Every fallible operation returns `Result<_, GuardianError>`.

5. **Serde for persistence** — `ProfilerSnapshot` is the serializable representation. Raw `Duration` fields are stored as `u64` microseconds for portability.
