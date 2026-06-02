//! Multi-format reporter for profiler data.
//!
//! Provides structured export in JSON, Prometheus, and CSV formats
//! for integration with dashboards, monitoring systems, and analysis tools.

use std::fmt;

use crate::profiler::{RenderProfiler, TrendReport};

/// Builder for exporting profiler data in multiple formats.
pub struct Reporter<'a> {
    profiler: &'a RenderProfiler,
    trend: Option<&'a TrendReport>,
}

impl<'a> Reporter<'a> {
    /// Create a reporter from the current profiler state.
    pub fn from_profiler(profiler: &'a RenderProfiler) -> Self {
        Self {
            profiler,
            trend: None,
        }
    }

    /// Attach trend data (from `Profiler::compare`) for richer exports.
    pub fn with_trend(mut self, trend: &'a TrendReport) -> Self {
        self.trend = Some(trend);
        self
    }

    /// Export as structured JSON.
    pub fn to_json(&self) -> String {
        let p = self.profiler;
        let budget = p.budget();

        let mut json = String::from("{");
        json.push_str(&format!("\"frame_number\": {},", p.last_frame()));

        if let Some(t) = p.last_frame_time() {
            json.push_str(&format!("\"last_frame_time_us\": {},", t.as_micros()));
        }

        json.push_str(&format!(
            "\"budget\": {{\"max_render_time_us\": {}, \"max_diff_cells\": {}, \"max_widget_depth\": {}}},",
            budget.max_render_time.as_micros(),
            budget.max_diff_cells,
            budget.max_widget_depth
        ));

        // Widgets
        json.push_str("\"widgets\": [");
        let mut sorted: Vec<_> = p.widget_stats().values().collect();
        sorted.sort_by_key(|w| &w.name);
        for (i, w) in sorted.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push('{');
            json.push_str(&format!("\"name\": {:?},", w.name));
            json.push_str(&format!("\"total_time_us\": {},", w.total_time_us));
            json.push_str(&format!("\"render_count\": {},", w.render_count));
            json.push_str(&format!("\"last_cells\": {},", w.last_cells));
            json.push_str(&format!("\"peak_time_us\": {},", w.peak_time_us));
            json.push_str(&format!("\"avg_time_us\": {},", w.avg_time().as_micros()));
            json.push_str(&format!("\"last_was_full_redraw\": {},", w.last_was_full_redraw));
            json.push_str(&format!("\"full_redraw_allowed\": {}", w.full_redraw_allowed));
            json.push('}');
        }
        json.push_str("],");

        // Findings
        let findings = p.last_findings();
        json.push_str("\"findings\": [");
        for (i, f) in findings.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push('{');
            json.push_str(&format!("\"frame\": {},", f.frame));
            json.push_str(&format!("\"severity\": \"{}\",", f.severity));
            json.push_str(&format!("\"message\": \"{}\"", f.category));
            json.push('}');
        }
        json.push_str("],");

        // Violations
        let violations = p.last_violations();
        json.push_str("\"violations\": [");
        for (i, v) in violations.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!("\"{}\"", v));
        }
        json.push(']');

        // Trend (if attached)
        if let Some(trend) = self.trend {
            json.push_str(",\"trend\": {");
            json.push_str(&format!("\"significant_degradation\": {},", trend.significant_degradation));
            json.push_str("\"degraded\": [");
            for (i, d) in trend.degraded.iter().enumerate() {
                if i > 0 { json.push(','); }
                json.push('{');
                json.push_str(&format!("\"name\": {:?},", d.name));
                json.push_str(&format!("\"previous_avg_us\": {},", d.previous_avg_us));
                json.push_str(&format!("\"current_avg_us\": {},", d.current_avg_us));
                json.push_str(&format!("\"change_percent\": {:.1}", d.change_percent));
                json.push('}');
            }
            json.push_str("],\"improved\": [");
            for (i, imp) in trend.improved.iter().enumerate() {
                if i > 0 { json.push(','); }
                json.push('{');
                json.push_str(&format!("\"name\": {:?},", imp.name));
                json.push_str(&format!("\"previous_avg_us\": {},", imp.previous_avg_us));
                json.push_str(&format!("\"current_avg_us\": {},", imp.current_avg_us));
                json.push_str(&format!("\"change_percent\": {:.1}", imp.change_percent));
                json.push('}');
            }
            json.push(']');
            json.push('}');
        }

        json.push('}');
        json
    }

    /// Export as Prometheus exposition-format metrics.
    ///
    /// Each widget emits:
    /// - `guardian_widget_total_time_us` (counter)
    /// - `guardian_widget_render_count` (counter)
    /// - `guardian_widget_peak_time_us` (gauge)
    /// - `guardian_widget_last_cells` (gauge)
    ///
    /// Frame-level:
    /// - `guardian_frame_time_us` (gauge)
    /// - `guardian_frame_total_cells` (gauge)
    /// - `guardian_findings_total` (gauge, by severity)
    pub fn to_prometheus(&self) -> String {
        let p = self.profiler;
        let mut out = String::new();

        // Frame-level metrics
        if let Some(t) = p.last_frame_time() {
            out.push_str("# HELP guardian_frame_time_us Total time for the last profiled frame in microseconds.\n");
            out.push_str("# TYPE guardian_frame_time_us gauge\n");
            out.push_str(&format!(
                "guardian_frame_time_us {}\n\n",
                t.as_micros()
            ));
        }

        out.push_str("# HELP guardian_frame_total_cells Total cells written in the last frame.\n");
        out.push_str("# TYPE guardian_frame_total_cells gauge\n");
        out.push_str(&format!(
            "guardian_frame_total_cells {}\n\n",
            p.last_frame_total_cells()
        ));

        // Findings by severity
        let findings = p.last_findings();
        let warnings = findings.iter().filter(|f| matches!(f.severity, crate::detector::Severity::Warning)).count();
        let critical = findings.iter().filter(|f| matches!(f.severity, crate::detector::Severity::Critical)).count();
        let hints = findings.len() - warnings - critical;

        out.push_str("# HELP guardian_findings_total Number of waste findings by severity.\n");
        out.push_str("# TYPE guardian_findings_total gauge\n");
        out.push_str(&format!("guardian_findings_total{{severity=\"critical\"}} {critical}\n"));
        out.push_str(&format!("guardian_findings_total{{severity=\"warning\"}} {warnings}\n"));
        out.push_str(&format!("guardian_findings_total{{severity=\"hint\"}} {hints}\n\n"));

        // Per-widget metrics
        out.push_str("# HELP guardian_widget_total_time_us Total render time for this widget in microseconds.\n");
        out.push_str("# TYPE guardian_widget_total_time_us counter\n");
        let mut sorted: Vec<_> = p.widget_stats().values().collect();
        sorted.sort_by_key(|w| &w.name);
        for w in &sorted {
            let label = &w.name;
            out.push_str(&format!(
                "guardian_widget_total_time_us{{widget=\"{label}\"}} {}\n",
                w.total_time_us
            ));
        }
        out.push('\n');

        out.push_str("# HELP guardian_widget_render_count Number of times this widget has been rendered.\n");
        out.push_str("# TYPE guardian_widget_render_count counter\n");
        for w in &sorted {
            let label = &w.name;
            out.push_str(&format!(
                "guardian_widget_render_count{{widget=\"{label}\"}} {}\n",
                w.render_count
            ));
        }
        out.push('\n');

        out.push_str("# HELP guardian_widget_peak_time_us Peak render time for this widget in microseconds.\n");
        out.push_str("# TYPE guardian_widget_peak_time_us gauge\n");
        for w in &sorted {
            let label = &w.name;
            out.push_str(&format!(
                "guardian_widget_peak_time_us{{widget=\"{label}\"}} {}\n",
                w.peak_time_us
            ));
        }
        out.push('\n');

        out.push_str("# HELP guardian_widget_last_cells Cells written by this widget in the last render.\n");
        out.push_str("# TYPE guardian_widget_last_cells gauge\n");
        for w in &sorted {
            let label = &w.name;
            out.push_str(&format!(
                "guardian_widget_last_cells{{widget=\"{label}\"}} {}\n",
                w.last_cells
            ));
        }

        out
    }

    /// Export as CSV.
    ///
    /// Columns: widget, total_time_us, render_count, last_cells, peak_time_us, avg_time_us, full_redraw
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("widget,total_time_us,render_count,last_cells,peak_time_us,avg_time_us,full_redraw\n");
        let mut sorted: Vec<_> = self.profiler.widget_stats().values().collect();
        sorted.sort_by_key(|w| &w.name);
        for w in &sorted {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                w.name,
                w.total_time_us,
                w.render_count,
                w.last_cells,
                w.peak_time_us,
                w.avg_time().as_micros(),
                w.last_was_full_redraw
            ));
        }
        csv
    }
}

impl fmt::Display for Reporter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_json())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::FrameBudget;
    use std::thread;
    use std::time::Duration;

    fn make_profiler() -> RenderProfiler {
        let mut p = RenderProfiler::new(FrameBudget::for_60fps());
        p.begin_frame();
        p.begin_widget("Header");
        thread::sleep(Duration::from_micros(500));
        p.end_widget(200);
        p.begin_widget("Body");
        thread::sleep(Duration::from_micros(200));
        p.end_widget(2000);
        p.end_frame();
        p
    }

    #[test]
    fn json_export_structure() {
        let p = make_profiler();
        let r = Reporter::from_profiler(&p);
        let json = r.to_json();
        assert!(json.contains("\"frame_number\": 1"));
        assert!(json.contains("\"Header\""));
        assert!(json.contains("\"Body\""));
        assert!(json.contains("\"budget\":"));
        assert!(json.contains("\"findings\":"));
    }

    #[test]
    fn prometheus_export_format() {
        let p = make_profiler();
        let r = Reporter::from_profiler(&p);
        let prom = r.to_prometheus();
        assert!(prom.contains("guardian_frame_time_us"));
        assert!(prom.contains("guardian_widget_total_time_us{widget=\"Header\"}"));
        assert!(prom.contains("# TYPE guardian_widget_render_count counter"));
        assert!(prom.contains("guardian_frame_total_cells"));
    }

    #[test]
    fn csv_export_format() {
        let p = make_profiler();
        let r = Reporter::from_profiler(&p);
        let csv = r.to_csv();
        assert!(csv.starts_with("widget,total_time_us,render_count"));
        assert!(csv.contains("Header,"));
        assert!(csv.contains("Body,"));
        // Should have header + 2 data rows
        assert_eq!(csv.lines().count(), 3);
    }
}
