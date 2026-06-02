# Production Log — ratatui-guardian v0.2.0

**Date:** 2026-06-02
**Module:** ratatui-guardian
**Version:** 0.1.0 → 0.2.0
**Language:** Rust
**Registry:** crates.io

## What Took the Most Time?

**Clippy compliance.** The v0.1 codebase had several clippy issues (manual_checked_ops, unnecessary_sort_by, single_char_add_str, useless_format). Fixing these required careful edits in reporter.rs and profiler.rs. Not hard, just fiddly.

**Serde migration.** Converting `PerWidgetStats` fields from `Duration` to `u64` microseconds (for serializability) required updating all references across profiler.rs and detector.rs tests. Mechanical but error-prone.

## What Was Mechanical vs. Required Thinking?

**Mechanical (template-able):**
- GuardianError enum — trivial error hierarchy, same pattern as conservation-guardian
- CI/CD workflow — standard GitHub Actions with cargo test + clippy + rustfmt
- CHANGELOG / CONTRIBUTING — OSS boilerplate
- Examples — once the API is stable, these write themselves
- Reporter.to_prometheus() and to_csv() — format-specific serialization
- GuardianBuffer — simple counter wrapper

**Required thinking:**
- `ProfilerSnapshot` serialization design — deciding to store `u64` microseconds instead of Duration (not directly serializable). This affected the entire PerWidgetStats struct.
- `Profiler::compare()` significance thresholds — what constitutes a meaningful degradation? Settled on configurable threshold with 0.25 (25%) as default.
- `GuardianBuffer` API — should it wrap ratatui's Buffer directly or be agnostic? Chose agnostic to avoid a ratatui dependency. Users call `set_cell()` / `set_string()` from their render code.
- Workspace isolation — the guardian crate sits inside the ratatui workspace but needs its own `[workspace]` in Cargo.toml to opt out of the parent workspace.

## Templates/Patterns for the Next Module

1. **Error enum pattern:** `GuardianError` with variants for Io (path + source), Json (context + source), NoData, InvalidConfig, ComparisonFailed. Flat hierarchy, one level deep.

2. **Reporter pattern:** Class/struct taking profiler reference, with `to_FORMAT()` methods. Identical to conservation-guardian's Reporter.

3. **Persistence pattern:** `save_json()` / `load_json()` on the main profiler. Snapshot struct is the serializable mirror of runtime state. Store durations as microseconds.

4. **Adapter pattern:** GuardianBuffer is a simple counter, not a trait. This avoids generics complexity while still providing automatic tracking.

5. **CI/CD boilerplate:** checkout@v4 + dtolnay/rust-toolchain@stable, three jobs (test, clippy, fmt). Standard.

## What Would I Do Differently Next Time?

1. **Design PerWidgetStats for serde from the start.** Converting Duration fields to u64 microseconds after the fact required touching detector tests. If I'd started with u64, it would have been zero-friction.

2. **Don't nest inside a workspace without planning.** The `[workspace]` opt-out is easy but I wasted a build cycle discovering it.

3. **Run clippy before finalizing the file.** I wrote reporter.rs with push_str("]") everywhere, then had to fix each one. Writing clippy-clean code from the start saves the cleanup pass.

4. **The compare() method** should accept a `CompareConfig` struct for thresholds rather than a raw f64. More extensible.

5. **Consider adding `#[non_exhaustive]` to all public enums.** Future-proofing for v0.3 additions without breaking changes.

## Stats

- **Files changed:** 17 (18 including PRODUCTION_LOG)
- **Lines added:** ~1,447
- **Tests:** 28 (all passing)
- **Clippy:** Clean (0 warnings)
- **Time:** ~12 minutes total
- **crates.io:** Published as ratatui-guardian v0.2.0
- **GitHub:** Pushed to SuperInstance/ratatui on `guardian` branch (commit 42091ae0)
