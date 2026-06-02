//! Per-widget render profiling.
//!
//! Tracks how long each widget takes to render, how many cells it writes,
//! and whether it's doing full redraws or incremental updates.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::budget::{BudgetViolation, FrameBudget};
use crate::detector::WasteDetector;
use crate::error::{GuardianError, Result};
use crate::report::ReportFormatter;

/// Statistics collected for a single widget across its lifetime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerWidgetStats {
    /// Widget name (the label you passed to `begin_widget`).
    pub name: String,
    /// Total render time accumulated across all frames.
    pub total_time_us: u64,
    /// Number of times this widget has been rendered.
    pub render_count: u64,
    /// Cells written in the most recent render.
    pub last_cells: usize,
    /// Peak render time for a single call (microseconds).
    pub peak_time_us: u64,
    /// Cells written on the previous render (for diff tracking).
    pub prev_cells: usize,
    /// Whether the last render was a "full redraw" (cells changed significantly).
    pub last_was_full_redraw: bool,
    /// Whether this widget is allowed to do full redraws without triggering
    /// the full-redraw heuristic (e.g. full-screen widgets).
    pub full_redraw_allowed: bool,
}

impl PerWidgetStats {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            total_time_us: 0,
            render_count: 0,
            last_cells: 0,
            peak_time_us: 0,
            prev_cells: 0,
            last_was_full_redraw: false,
            full_redraw_allowed: false,
        }
    }

    /// Average render time per call.
    pub fn avg_time(&self) -> Duration {
        // Clippy: manual_checked_ops — use checked_div
        #[allow(clippy::manual_checked_ops)]
        if self.render_count == 0 {
            Duration::ZERO
        } else {
            Duration::from_micros(self.total_time_us / self.render_count)
        }
    }

    /// Total time as a Duration.
    pub fn total_time(&self) -> Duration {
        Duration::from_micros(self.total_time_us)
    }

    /// Peak time as a Duration.
    pub fn peak_time(&self) -> Duration {
        Duration::from_micros(self.peak_time_us)
    }

    /// What fraction of total frame time this widget consumed (0.0 – 1.0).
    pub fn fraction_of(&self, total: Duration) -> f64 {
        if total.is_zero() {
            0.0
        } else {
            self.total_time().as_secs_f64() / total.as_secs_f64()
        }
    }
}

/// Serializable snapshot of profiler state for persistence.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProfilerSnapshot {
    pub frame_number: u64,
    pub budget_max_render_time_us: u64,
    pub budget_max_diff_cells: usize,
    pub budget_max_widget_depth: usize,
    pub widget_stats: Vec<PerWidgetStats>,
    pub unmatched_end_widget_count: u64,
    pub last_frame_max_depth: usize,
    pub last_frame_time_us: Option<u64>,
    pub last_frame_total_cells: usize,
}

/// A single completed frame's data.
#[derive(Debug, Clone)]
pub(crate) struct FrameRecord {
    pub total_time: Duration,
    pub widget_times: Vec<(String, Duration, usize)>,
    pub violations: Vec<BudgetViolation>,
}

/// Trend analysis result from comparing two profiler states.
#[derive(Debug, Clone)]
pub struct TrendReport {
    /// Widgets whose average render time increased.
    pub degraded: Vec<WidgetTrend>,
    /// Widgets whose average render time decreased.
    pub improved: Vec<WidgetTrend>,
    /// Widgets only present in one of the two snapshots.
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Whether any degradation exceeds the significance threshold.
    pub significant_degradation: bool,
}

/// Trend data for a single widget across two profiler snapshots.
#[derive(Debug, Clone)]
pub struct WidgetTrend {
    pub name: String,
    pub previous_avg_us: u64,
    pub current_avg_us: u64,
    pub change_percent: f64,
}

/// The main profiler. Owns the budget and collects per-frame / per-widget data.
pub struct RenderProfiler {
    budget: FrameBudget,
    frame_number: u64,
    frame_start: Option<Instant>,
    widget_stack: Vec<(String, Instant)>,
    current_frame_widgets: Vec<(String, Duration, usize)>,
    widget_stats: HashMap<String, PerWidgetStats>,
    history: Vec<FrameRecord>,
    detector: WasteDetector,
    max_history: usize,
    /// Number of times `end_widget` was called with an empty widget stack.
    unmatched_end_widget_count: u64,
    /// Deepest actual nesting depth seen in the current frame.
    current_frame_max_depth: usize,
    /// Whether a depth violation was detected in the current frame.
    depth_violation_seen: Option<usize>,
}

impl RenderProfiler {
    /// Create a new profiler with the given budget.
    pub fn new(budget: FrameBudget) -> Self {
        Self {
            budget,
            frame_number: 0,
            frame_start: None,
            widget_stack: Vec::new(),
            current_frame_widgets: Vec::new(),
            widget_stats: HashMap::new(),
            history: Vec::new(),
            detector: WasteDetector::new(),
            max_history: 120,
            unmatched_end_widget_count: 0,
            current_frame_max_depth: 0,
            depth_violation_seen: None,
        }
    }

    /// The configured budget.
    pub fn budget(&self) -> &FrameBudget {
        &self.budget
    }

    /// Begin a new frame.
    pub fn begin_frame(&mut self) {
        self.frame_number += 1;
        self.frame_start = Some(Instant::now());
        self.current_frame_widgets.clear();
        self.current_frame_max_depth = 0;
        self.depth_violation_seen = None;
    }

    /// Begin timing a widget. Nesting is tracked for depth checks.
    pub fn begin_widget(&mut self, name: &str) {
        let depth = self.widget_stack.len() + 1;
        if depth > self.current_frame_max_depth {
            self.current_frame_max_depth = depth;
        }
        if self.budget.check_depth(depth).is_some() {
            self.depth_violation_seen = Some(depth);
        }
        self.widget_stack
            .push((name.to_string(), Instant::now()));
    }

    /// End timing a widget. `cells_written` is the number of terminal cells this widget touched.
    pub fn end_widget(&mut self, cells_written: usize) {
        if self.widget_stack.is_empty() {
            self.unmatched_end_widget_count += 1;
            return;
        }
        if let Some((name, start)) = self.widget_stack.pop() {
            let elapsed = start.elapsed();
            self.current_frame_widgets
                .push((name.clone(), elapsed, cells_written));

            let stats = self
                .widget_stats
                .entry(name.clone())
                .or_insert_with(|| PerWidgetStats::new(&name));

            let full_redraw = if stats.render_count > 0 {
                let diff = (cells_written as i64 - stats.last_cells as i64).unsigned_abs() as usize;
                diff > (stats.last_cells / 2)
            } else {
                false
            };

            stats.prev_cells = stats.last_cells;
            stats.last_cells = cells_written;
            stats.last_was_full_redraw = full_redraw;
            stats.total_time_us += elapsed.as_micros() as u64;
            stats.render_count += 1;
            let elapsed_us = elapsed.as_micros() as u64;
            if elapsed_us > stats.peak_time_us {
                stats.peak_time_us = elapsed_us;
            }
        }
    }

    /// End the current frame. Returns total frame time.
    pub fn end_frame(&mut self) -> Duration {
        let total = self
            .frame_start
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);

        let mut violations = Vec::new();
        if let Some(v) = self.budget.check_time(total) {
            violations.push(v);
        }

        let total_cells: usize = self.current_frame_widgets.iter().map(|w| w.2).sum();
        if let Some(v) = self.budget.check_diff(total_cells) {
            violations.push(v);
        }

        let findings = self.detector.detect(
            self.frame_number,
            total,
            &self.current_frame_widgets,
            &self.widget_stats,
            &self.budget,
            self.current_frame_max_depth,
        );

        let record = FrameRecord {
            total_time: total,
            widget_times: self.current_frame_widgets.clone(),
            violations,
        };

        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(record);

        self.detector.stash_findings(self.frame_number, findings);

        total
    }

    /// Generate a human-readable report for the last frame.
    pub fn report(&self) -> ReportFormatter<'_> {
        ReportFormatter::new(self)
    }

    /// Per-widget stats.
    pub fn widget_stats(&self) -> &HashMap<String, PerWidgetStats> {
        &self.widget_stats
    }

    /// Last frame number.
    pub fn last_frame(&self) -> u64 {
        self.frame_number
    }

    /// Last frame total time.
    pub fn last_frame_time(&self) -> Option<Duration> {
        self.history.last().map(|r| r.total_time)
    }

    /// Violations from the last frame.
    pub fn last_violations(&self) -> &[BudgetViolation] {
        self.history
            .last()
            .map(|r| r.violations.as_slice())
            .unwrap_or(&[])
    }

    /// Waste findings from the last frame.
    pub fn last_findings(&self) -> &[crate::detector::WasteFinding] {
        self.detector.last_findings()
    }

    /// Access the history (most recent last).
    pub(crate) fn history(&self) -> &[FrameRecord] {
        &self.history
    }

    /// Get the total cells from the last frame's widgets.
    pub fn last_frame_total_cells(&self) -> usize {
        self.history
            .last()
            .map(|r| r.widget_times.iter().map(|w| w.2).sum())
            .unwrap_or(0)
    }

    /// Number of unmatched `end_widget` calls detected.
    pub fn unmatched_end_widget_count(&self) -> u64 {
        self.unmatched_end_widget_count
    }

    /// Deepest actual nesting depth seen in the last frame.
    pub fn last_frame_max_depth(&self) -> usize {
        self.current_frame_max_depth
    }

    /// Mark a widget as allowed to do full redraws without triggering the heuristic.
    pub fn set_full_redraw_allowed(&mut self, widget_name: &str, allowed: bool) {
        if let Some(stats) = self.widget_stats.get_mut(widget_name) {
            stats.full_redraw_allowed = allowed;
        } else {
            let mut stats = PerWidgetStats::new(widget_name);
            stats.full_redraw_allowed = allowed;
            self.widget_stats.insert(widget_name.to_string(), stats);
        }
    }

    /// Reset all accumulated profiler state, keeping the same budget.
    pub fn reset(&mut self) {
        self.frame_number = 0;
        self.frame_start = None;
        self.widget_stack.clear();
        self.current_frame_widgets.clear();
        self.widget_stats.clear();
        self.history.clear();
        self.detector = WasteDetector::new();
        self.unmatched_end_widget_count = 0;
        self.current_frame_max_depth = 0;
        self.depth_violation_seen = None;
    }

    // ── Persistence ──────────────────────────────────────────────────

    /// Save profiler state to a JSON file.
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<()> {
        let snapshot = self.to_snapshot();
        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| GuardianError::Json {
            context: "serialize profiler snapshot".into(),
            source: e,
        })?;
        fs::write(path.as_ref(), json).map_err(|e| GuardianError::Io {
            path: path.as_ref().to_path_buf(),
            source: e,
        })?;
        Ok(())
    }

    /// Load profiler state from a JSON file. Budget is supplied by the caller.
    pub fn load_json(path: impl AsRef<Path>, budget: FrameBudget) -> Result<Self> {
        let data = fs::read_to_string(path.as_ref()).map_err(|e| GuardianError::Io {
            path: path.as_ref().to_path_buf(),
            source: e,
        })?;
        let snapshot: ProfilerSnapshot =
            serde_json::from_str(&data).map_err(|e| GuardianError::Json {
                context: "deserialize profiler snapshot".into(),
                source: e,
            })?;

        let mut widget_stats = HashMap::new();
        for ws in snapshot.widget_stats {
            widget_stats.insert(ws.name.clone(), ws);
        }

        Ok(Self {
            budget,
            frame_number: snapshot.frame_number,
            frame_start: None,
            widget_stack: Vec::new(),
            current_frame_widgets: Vec::new(),
            widget_stats,
            history: Vec::new(),
            detector: WasteDetector::new(),
            max_history: 120,
            unmatched_end_widget_count: snapshot.unmatched_end_widget_count,
            current_frame_max_depth: snapshot.last_frame_max_depth,
            depth_violation_seen: None,
        })
    }

    fn to_snapshot(&self) -> ProfilerSnapshot {
        ProfilerSnapshot {
            frame_number: self.frame_number,
            budget_max_render_time_us: self.budget.max_render_time.as_micros() as u64,
            budget_max_diff_cells: self.budget.max_diff_cells,
            budget_max_widget_depth: self.budget.max_widget_depth,
            widget_stats: self.widget_stats.values().cloned().collect(),
            unmatched_end_widget_count: self.unmatched_end_widget_count,
            last_frame_max_depth: self.current_frame_max_depth,
            last_frame_time_us: self.last_frame_time().map(|t| t.as_micros() as u64),
            last_frame_total_cells: self.last_frame_total_cells(),
        }
    }

    // ── Trend analysis ───────────────────────────────────────────────

    /// Compare this profiler's state against a previous one to detect trends.
    ///
    /// `significance_threshold` is the percentage change (e.g. `0.25` means 25%)
    /// above which degradation is flagged as significant.
    pub fn compare(&self, previous: &RenderProfiler, significance_threshold: f64) -> Result<TrendReport> {
        let mut degraded = Vec::new();
        let mut improved = Vec::new();
        let mut added = Vec::new();
        let mut removed = Vec::new();

        // Check for widgets in current but not previous
        for name in self.widget_stats.keys() {
            if !previous.widget_stats.contains_key(name) {
                added.push(name.clone());
            }
        }

        // Check for widgets in previous but not current
        for name in previous.widget_stats.keys() {
            if !self.widget_stats.contains_key(name) {
                removed.push(name.clone());
            }
        }

        // Compare shared widgets
        for (name, current_stats) in &self.widget_stats {
            if let Some(prev_stats) = previous.widget_stats.get(name) {
                if current_stats.render_count == 0 || prev_stats.render_count == 0 {
                    continue;
                }
                let current_avg = current_stats.total_time_us / current_stats.render_count;
                let prev_avg = prev_stats.total_time_us / prev_stats.render_count;

                if prev_avg == 0 {
                    continue;
                }

                let change = (current_avg as f64 - prev_avg as f64) / prev_avg as f64 * 100.0;
                let trend = WidgetTrend {
                    name: name.clone(),
                    previous_avg_us: prev_avg,
                    current_avg_us: current_avg,
                    change_percent: change,
                };

                if current_avg > prev_avg {
                    degraded.push(trend);
                } else if current_avg < prev_avg {
                    improved.push(trend);
                }
            }
        }

        let significant_degradation = degraded
            .iter()
            .any(|t| t.change_percent.abs() > significance_threshold * 100.0);

        // Sort by magnitude
        degraded.sort_by(|a, b| b.change_percent.partial_cmp(&a.change_percent).unwrap_or(std::cmp::Ordering::Equal));
        improved.sort_by(|a, b| a.change_percent.partial_cmp(&b.change_percent).unwrap_or(std::cmp::Ordering::Equal));

        Ok(TrendReport {
            degraded,
            improved,
            added,
            removed,
            significant_degradation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn basic_profiling_cycle() {
        let mut p = RenderProfiler::new(FrameBudget::for_60fps());

        p.begin_frame();
        p.begin_widget("Header");
        thread::sleep(Duration::from_millis(1));
        p.end_widget(200);
        p.begin_widget("Body");
        thread::sleep(Duration::from_millis(1));
        p.end_widget(2000);
        let total = p.end_frame();

        assert!(total >= Duration::from_millis(2));
        assert_eq!(p.last_frame(), 1);

        let stats = p.widget_stats();
        assert_eq!(stats.len(), 2);
        assert!(stats.get("Header").unwrap().render_count == 1);
        assert!(stats.get("Body").unwrap().last_cells == 2000);
    }

    #[test]
    fn detects_over_budget() {
        let budget = FrameBudget::new(Duration::from_millis(1), 10_000, 5);
        let mut p = RenderProfiler::new(budget);

        p.begin_frame();
        p.begin_widget("SlowWidget");
        thread::sleep(Duration::from_millis(5));
        p.end_widget(100);
        p.end_frame();

        assert!(!p.last_violations().is_empty());
    }

    #[test]
    fn full_redraw_detection() {
        let mut p = RenderProfiler::new(FrameBudget::for_60fps());

        p.begin_frame();
        p.begin_widget("Status");
        p.end_widget(100);
        p.end_frame();

        p.begin_frame();
        p.begin_widget("Status");
        p.end_widget(200);
        p.end_frame();

        let stats = p.widget_stats().get("Status").unwrap();
        assert!(stats.last_was_full_redraw);
    }

    #[test]
    fn unmatched_end_widget_is_counted() {
        let mut p = RenderProfiler::new(FrameBudget::for_60fps());
        p.begin_frame();
        p.end_widget(100);
        assert_eq!(p.unmatched_end_widget_count(), 1);
        p.end_widget(50);
        assert_eq!(p.unmatched_end_widget_count(), 2);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut p = RenderProfiler::new(FrameBudget::for_60fps());

        p.begin_frame();
        p.begin_widget("Widget");
        p.end_widget(100);
        p.end_frame();

        assert_eq!(p.last_frame(), 1);
        assert!(!p.widget_stats().is_empty());

        p.reset();

        assert_eq!(p.last_frame(), 0);
        assert!(p.widget_stats().is_empty());
        assert_eq!(p.unmatched_end_widget_count(), 0);
    }

    #[test]
    fn save_and_load_json_roundtrip() {
        let dir = std::env::temp_dir().join("guardian_test_save_load");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("profiler.json");

        let mut p = RenderProfiler::new(FrameBudget::for_60fps());
        p.begin_frame();
        p.begin_widget("Header");
        p.end_widget(200);
        p.end_frame();

        p.save_json(&path).unwrap();

        let loaded = RenderProfiler::load_json(&path, FrameBudget::for_60fps()).unwrap();
        assert_eq!(loaded.last_frame(), 1);
        assert_eq!(loaded.widget_stats().len(), 1);
        assert_eq!(
            loaded.widget_stats().get("Header").unwrap().last_cells,
            200
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn compare_detects_degradation() {
        let budget = FrameBudget::for_60fps();

        let mut prev = RenderProfiler::new(budget.clone());
        prev.begin_frame();
        prev.begin_widget("Widget");
        thread::sleep(Duration::from_micros(100));
        prev.end_widget(50);
        prev.end_frame();

        let mut curr = RenderProfiler::new(budget.clone());
        curr.begin_frame();
        curr.begin_widget("Widget");
        thread::sleep(Duration::from_millis(5));
        curr.end_widget(50);
        curr.end_frame();

        let trend = curr.compare(&prev, 0.1).unwrap();
        assert!(!trend.degraded.is_empty());
        assert!(trend.significant_degradation);
    }

    #[test]
    fn compare_detects_added_removed() {
        let budget = FrameBudget::for_60fps();

        let mut prev = RenderProfiler::new(budget.clone());
        prev.begin_frame();
        prev.begin_widget("A");
        prev.end_widget(10);
        prev.end_frame();

        let mut curr = RenderProfiler::new(budget.clone());
        curr.begin_frame();
        curr.begin_widget("B");
        curr.end_widget(10);
        curr.end_frame();

        let trend = curr.compare(&prev, 0.5).unwrap();
        assert!(trend.added.contains(&"B".to_string()));
        assert!(trend.removed.contains(&"A".to_string()));
    }

    #[test]
    fn full_redraw_allowed_can_be_set() {
        let mut p = RenderProfiler::new(FrameBudget::for_60fps());

        p.set_full_redraw_allowed("MyWidget", true);
        p.begin_frame();
        p.begin_widget("MyWidget");
        p.end_widget(100);
        p.end_frame();

        assert!(p.widget_stats().get("MyWidget").unwrap().full_redraw_allowed);

        p.set_full_redraw_allowed("MyWidget", false);
        assert!(!p.widget_stats().get("MyWidget").unwrap().full_redraw_allowed);
    }

    #[test]
    fn depth_tracking_is_accurate() {
        let mut p = RenderProfiler::new(FrameBudget::for_60fps());

        p.begin_frame();
        p.begin_widget("A");
        p.begin_widget("B");
        p.begin_widget("C");
        p.end_widget(10);
        p.end_widget(10);
        p.end_widget(10);
        p.end_frame();

        assert_eq!(p.last_frame_max_depth(), 3);
    }
}
