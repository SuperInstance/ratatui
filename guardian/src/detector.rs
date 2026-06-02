//! Waste detection.
//!
//! Identifies common TUI rendering anti-patterns:
//! - Full-screen redraws for tiny changes
//! - Excessively nested layouts
//! - Widgets that appear to allocate on every frame (heuristic)

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use crate::budget::FrameBudget;
use crate::profiler::PerWidgetStats;

/// What kind of waste was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasteCategory {
    /// A widget touched nearly every cell but likely only needed to update a few.
    FullRedrawForSmallChange {
        widget: String,
        cells_written: usize,
        estimated_needed: usize,
    },
    /// Layout nesting exceeds the configured depth limit.
    DeepNesting {
        depth: usize,
        limit: usize,
    },
    /// A widget's render time is suspiciously high for the number of cells,
    /// suggesting per-frame allocation (String, Vec, etc.).
    SuspectedAllocation {
        widget: String,
        render_time_us: u64,
        cells: usize,
    },
    /// A single widget dominates frame time.
    Hog {
        widget: String,
        fraction_percent: u64,
    },
}

impl fmt::Display for WasteCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FullRedrawForSmallChange {
                widget,
                cells_written,
                estimated_needed,
            } => write!(
                f,
                "{widget} wrote {cells_written} cells but likely only needed ~{estimated_needed}. \
                 It redraws the full area on every frame."
            ),
            Self::DeepNesting { depth, limit } => {
                write!(f, "layout nested {depth} levels deep (limit: {limit})")
            }
            Self::SuspectedAllocation {
                widget,
                render_time_us,
                cells,
            } => write!(
                f,
                "{widget} took {render_time_us}µs for only {cells} cells — \
                 likely allocating Strings or Vecs every frame"
            ),
            Self::Hog {
                widget,
                fraction_percent,
            } => write!(
                f,
                "{widget} consumed {fraction_percent}% of frame time"
            ),
        }
    }
}

/// A single waste finding for a specific frame.
#[derive(Debug, Clone)]
pub struct WasteFinding {
    pub frame: u64,
    pub category: WasteCategory,
    pub severity: Severity,
}

/// How bad is it, really.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Hint,
    Warning,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Hint => write!(f, "hint"),
            Severity::Warning => write!(f, "warning"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// The waste detector. Stateless between calls — all context comes from profiler data.
pub struct WasteDetector {
    last_findings: Vec<WasteFinding>,
}

impl WasteDetector {
    pub fn new() -> Self {
        Self {
            last_findings: Vec::new(),
        }
    }

    /// Analyze a frame's data and return findings.
    pub fn detect(
        &self,
        frame: u64,
        frame_time: Duration,
        widgets: &[(String, Duration, usize)],
        all_stats: &HashMap<String, PerWidgetStats>,
        budget: &FrameBudget,
        actual_max_depth: usize,
    ) -> Vec<WasteFinding> {
        let mut findings = Vec::new();
        let cfg = &budget.detection;

        for (name, time, cells) in widgets {
            // Check for hog: widget takes >configured fraction of frame time
            // Bug fix: use > 1µs guard for floating-point safety instead of is_zero()
            if frame_time > Duration::from_micros(1) {
                let fraction = time.as_secs_f64() / frame_time.as_secs_f64();
                if fraction > cfg.hog_fraction {
                    findings.push(WasteFinding {
                        frame,
                        category: WasteCategory::Hog {
                            widget: name.clone(),
                            fraction_percent: (fraction * 100.0) as u64,
                        },
                        severity: if fraction > 0.85 {
                            Severity::Critical
                        } else {
                            Severity::Warning
                        },
                    });
                }
            }

            // Check for full-redraw-for-small-change
            // Bug fix: skip if widget has full_redraw_allowed set
            if let Some(stats) = all_stats.get(name) {
                if !stats.full_redraw_allowed
                    && stats.render_count > 3
                    && stats.last_was_full_redraw
                    && *cells > cfg.full_redraw_cell_threshold
                {
                    let estimated = (*cells / 10).max(2);
                    findings.push(WasteFinding {
                        frame,
                        category: WasteCategory::FullRedrawForSmallChange {
                            widget: name.clone(),
                            cells_written: *cells,
                            estimated_needed: estimated,
                        },
                        severity: if *cells > 5000 {
                            Severity::Warning
                        } else {
                            Severity::Hint
                        },
                    });
                }
            }

            // Check for suspected allocation: uses configurable threshold
            let time_us = time.as_micros() as u64;
            if *cells > 0 && time_us > 0 {
                let us_per_cell = time_us / (*cells as u64).max(1);
                if us_per_cell > cfg.allocation_us_per_cell {
                    findings.push(WasteFinding {
                        frame,
                        category: WasteCategory::SuspectedAllocation {
                            widget: name.clone(),
                            render_time_us: time_us,
                            cells: *cells,
                        },
                        severity: if us_per_cell > cfg.allocation_us_per_cell * 4 {
                            Severity::Warning
                        } else {
                            Severity::Hint
                        },
                    });
                }
            }
        }

        // Bug fix: Deep nesting check now uses actual tracked depth, not total widget count
        if actual_max_depth > budget.max_widget_depth * cfg.deep_nesting_multiplier
            || actual_max_depth > budget.max_widget_depth
        {
            // Only emit if it's actually over the depth limit
            if actual_max_depth > budget.max_widget_depth {
                findings.push(WasteFinding {
                    frame,
                    category: WasteCategory::DeepNesting {
                        depth: actual_max_depth,
                        limit: budget.max_widget_depth,
                    },
                    severity: Severity::Hint,
                });
            }
        }

        findings
    }

    /// Stash findings for later retrieval (called by the profiler at end_frame).
    pub fn stash_findings(&mut self, _frame: u64, findings: Vec<WasteFinding>) {
        self.last_findings = findings;
    }

    /// Retrieve the findings from the last frame.
    pub fn last_findings(&self) -> &[WasteFinding] {
        &self.last_findings
    }
}

impl Default for WasteDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DetectionConfig;

    #[test]
    fn detects_hog() {
        let detector = WasteDetector::new();
        let budget = FrameBudget::for_60fps();
        let mut stats = HashMap::new();
        stats.insert(
            "StatusBar".to_string(),
            PerWidgetStats {
                name: "StatusBar".to_string(),
                total_time_us: Duration::from_millis(50).as_micros() as u64,
                render_count: 10,
                last_cells: 80,
                peak_time_us: Duration::from_millis(10).as_micros() as u64,
                prev_cells: 80,
                last_was_full_redraw: false,
                full_redraw_allowed: false,
            },
        );

        let widgets = vec![
            ("StatusBar".to_string(), Duration::from_millis(12), 80),
            ("Body".to_string(), Duration::from_millis(1), 2000),
        ];

        let findings = detector.detect(
            1,
            Duration::from_millis(14),
            &widgets,
            &stats,
            &budget,
            2, // actual_max_depth
        );

        let hogs: Vec<_> = findings
            .iter()
            .filter(|f| matches!(f.category, WasteCategory::Hog { .. }))
            .collect();
        assert!(!hogs.is_empty());
        assert!(matches!(hogs[0].severity, Severity::Critical));
    }

    // --- Bug fix tests ---

    #[test]
    fn division_by_zero_safe_with_tiny_frame_time() {
        // Bug #1: ensure no panic when frame_time is very small but non-zero
        let detector = WasteDetector::new();
        let budget = FrameBudget::for_60fps();
        let stats = HashMap::new();
        let widgets = vec![
            ("Widget".to_string(), Duration::from_nanos(500), 100),
        ];
        // frame_time of 0.5µs — should NOT panic
        let findings = detector.detect(
            1,
            Duration::from_nanos(500),
            &widgets,
            &stats,
            &budget,
            1,
        );
        // Should not trigger hog since frame_time is too small for reliable ratio
        assert!(findings.is_empty() || findings.iter().all(|f| !matches!(f.category, WasteCategory::Hog { .. })));
    }

    #[test]
    fn full_redraw_allowed_suppresses_heuristic() {
        // Bug #4: widget with full_redraw_allowed=true should not be flagged
        let detector = WasteDetector::new();
        let budget = FrameBudget::for_60fps();
        let mut stats = HashMap::new();
        stats.insert(
            "FullScreen".to_string(),
            PerWidgetStats {
                name: "FullScreen".to_string(),
                total_time_us: Duration::from_millis(5).as_micros() as u64,
                render_count: 10,
                last_cells: 2000,
                peak_time_us: Duration::from_millis(1).as_micros() as u64,
                prev_cells: 1000,
                last_was_full_redraw: true,
                full_redraw_allowed: true, // whitelisted
            },
        );
        let widgets = vec![
            ("FullScreen".to_string(), Duration::from_micros(500), 2000),
        ];
        let findings = detector.detect(
            1,
            Duration::from_millis(1),
            &widgets,
            &stats,
            &budget,
            1,
        );
        assert!(findings.iter().all(|f| !matches!(f.category, WasteCategory::FullRedrawForSmallChange { .. })));
    }

    #[test]
    fn deep_nesting_uses_actual_depth_not_widget_count() {
        // Bug #3: 10 flat widgets should NOT trigger deep nesting if depth is 1
        let detector = WasteDetector::new();
        let budget = FrameBudget::for_60fps(); // max_widget_depth = 5
        let stats = HashMap::new();
        let widgets: Vec<(String, Duration, usize)> = (0..10)
            .map(|i| (format!("Widget{i}"), Duration::from_micros(100), 50))
            .collect();
        // actual_max_depth = 1 (all flat), even though there are 10 widgets
        let findings = detector.detect(
            1,
            Duration::from_millis(1),
            &widgets,
            &stats,
            &budget,
            1, // actual depth is 1
        );
        assert!(findings.iter().all(|f| !matches!(f.category, WasteCategory::DeepNesting { .. })));
    }

    #[test]
    fn deep_nesting_flags_when_actual_depth_exceeds_limit() {
        // Bug #3: actual depth > limit should trigger
        let detector = WasteDetector::new();
        let budget = FrameBudget::for_60fps(); // max_widget_depth = 5
        let stats = HashMap::new();
        let widgets = vec![
            ("A".to_string(), Duration::from_micros(100), 50),
        ];
        let findings = detector.detect(
            1,
            Duration::from_millis(1),
            &widgets,
            &stats,
            &budget,
            8, // actual depth is 8 > max of 5
        );
        let nesting: Vec<_> = findings
            .iter()
            .filter(|f| matches!(f.category, WasteCategory::DeepNesting { .. }))
            .collect();
        assert_eq!(nesting.len(), 1);
        if let WasteCategory::DeepNesting { depth, limit } = &nesting[0].category {
            assert_eq!(*depth, 8);
            assert_eq!(*limit, 5);
        }
    }

    #[test]
    fn configurable_hog_fraction() {
        // Bug #5/#6: custom hog_fraction should be respected
        let detector = WasteDetector::new();
        let detection = DetectionConfig::with(
            0.9,  // hog_fraction = 90%
            500,
            50,
            2,
        );
        let budget = FrameBudget::with_detection(
            Duration::from_millis(16),
            10_000,
            5,
            detection,
        );
        let stats = HashMap::new();
        let widgets = vec![
            ("Widget".to_string(), Duration::from_millis(12), 100),
        ];
        // 12/14 ≈ 85.7%, below the 90% threshold
        let findings = detector.detect(
            1,
            Duration::from_millis(14),
            &widgets,
            &stats,
            &budget,
            1,
        );
        assert!(findings.iter().all(|f| !matches!(f.category, WasteCategory::Hog { .. })));
    }

    #[test]
    fn configurable_allocation_threshold() {
        // Bug #5/#6: custom allocation_us_per_cell should be respected
        let detector = WasteDetector::new();
        let detection = DetectionConfig::with(
            0.6,
            500,
            200, // threshold = 200µs/cell (higher than default 50)
            2,
        );
        let budget = FrameBudget::with_detection(
            Duration::from_millis(16),
            10_000,
            5,
            detection,
        );
        let stats = HashMap::new();
        // 100µs for 1 cell = 100µs/cell — should NOT trigger at 200 threshold
        let widgets = vec![
            ("Widget".to_string(), Duration::from_micros(100), 1),
        ];
        let findings = detector.detect(
            1,
            Duration::from_millis(1),
            &widgets,
            &stats,
            &budget,
            1,
        );
        assert!(findings.iter().all(|f| !matches!(f.category, WasteCategory::SuspectedAllocation { .. })));
    }
}
