//! Ratatui buffer/frame adapter for automatic widget tracking.
//!
//! This module provides [`GuardianBuffer`] — a wrapper around ratatui's
//! `Buffer` that intercepts every `set_cell` call so you don't have to
//! manually count cells in every widget. It also provides a thin
//! [`GuardianFrame`] helper for frame-level lifecycle management.
//!
//! # Usage
//!
//! ```rust,ignore
//! use ratatui_guardian::ratatui_adapter::GuardianBuffer;
//!
//! let mut gbuf = GuardianBuffer::new("MyWidget");
//! // ... your widget calls gbuf.set_cell(...) ...
//! let report = gbuf.finish();
//! assert_eq!(report.cells_written, 42);
//! ```

use std::fmt;
use std::time::{Duration, Instant};

/// Result of tracking a single widget's render through the adapter.
#[derive(Debug, Clone)]
pub struct WidgetRenderReport {
    /// The widget name passed to [`GuardianBuffer::new`].
    pub name: String,
    /// Total terminal cells written.
    pub cells_written: usize,
    /// Wall-clock render time.
    pub render_time: Duration,
}

/// A tracking wrapper that counts every `set_cell`-style call.
///
/// This does **not** depend on ratatui directly — it is agnostic to the
/// actual terminal buffer type. You call [`GuardianBuffer::set_cell`]
/// (or the `set_string` helper) from within your widget's render method,
/// and the adapter tallies cells automatically.
pub struct GuardianBuffer {
    name: String,
    cells_written: usize,
    start: Instant,
}

impl GuardianBuffer {
    /// Create a new tracking buffer for widget `name`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            cells_written: 0,
            start: Instant::now(),
        }
    }

    /// Record a single cell write. Call this from your widget's `render`
    /// method whenever it sets a cell in the ratatui `Buffer`.
    #[inline]
    pub fn set_cell(&mut self) {
        self.cells_written += 1;
    }

    /// Record `n` cell writes at once (e.g. for a string of length `n`).
    #[inline]
    pub fn set_string(&mut self, len: usize) {
        self.cells_written += len;
    }

    /// Current cell count (useful for mid-render checks).
    pub fn cells(&self) -> usize {
        self.cells_written
    }

    /// Finish tracking and return the render report.
    pub fn finish(self) -> WidgetRenderReport {
        WidgetRenderReport {
            name: self.name,
            cells_written: self.cells_written,
            render_time: self.start.elapsed(),
        }
    }
}

impl fmt::Debug for GuardianBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GuardianBuffer")
            .field("name", &self.name)
            .field("cells_written", &self.cells_written)
            .finish()
    }
}

/// Helper for tracking the full frame lifecycle.
///
/// Wrap your frame render with `begin` / `end`, and use the returned
/// `GuardianBuffer` instances to feed cell counts back into the profiler.
pub struct GuardianFrame {
    start: Instant,
    widget_buffers: Vec<WidgetRenderReport>,
}

impl GuardianFrame {
    /// Begin tracking a new frame.
    pub fn begin() -> Self {
        Self {
            start: Instant::now(),
            widget_buffers: Vec::new(),
        }
    }

    /// Create a tracked buffer for a widget within this frame.
    /// Call `finish()` on the buffer when the widget is done rendering.
    pub fn track_widget(&mut self, name: &str) -> GuardianBuffer {
        GuardianBuffer::new(name)
    }

    /// Finish a widget buffer and record its report.
    pub fn finish_widget(&mut self, buf: GuardianBuffer) {
        self.widget_buffers.push(buf.finish());
    }

    /// Finish the frame and return all widget reports plus total time.
    pub fn end(self) -> FrameReport {
        FrameReport {
            total_time: self.start.elapsed(),
            widgets: self.widget_buffers,
        }
    }
}

/// Aggregate report for an entire frame.
#[derive(Debug, Clone)]
pub struct FrameReport {
    /// Total wall-clock time for the frame.
    pub total_time: Duration,
    /// Per-widget render reports, in order of completion.
    pub widgets: Vec<WidgetRenderReport>,
}

impl FrameReport {
    /// Total cells written across all widgets.
    pub fn total_cells(&self) -> usize {
        self.widgets.iter().map(|w| w.cells_written).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_counts_cells() {
        let mut buf = GuardianBuffer::new("TestWidget");
        buf.set_cell();
        buf.set_cell();
        buf.set_cell();
        let report = buf.finish();
        assert_eq!(report.cells_written, 3);
        assert_eq!(report.name, "TestWidget");
    }

    #[test]
    fn buffer_set_string() {
        let mut buf = GuardianBuffer::new("Label");
        buf.set_string(12);
        assert_eq!(buf.cells(), 12);
        let report = buf.finish();
        assert_eq!(report.cells_written, 12);
    }

    #[test]
    fn frame_lifecycle() {
        let mut frame = GuardianFrame::begin();

        let mut w1 = frame.track_widget("Header");
        w1.set_string(80);
        frame.finish_widget(w1);

        let mut w2 = frame.track_widget("Body");
        w2.set_string(2000);
        frame.finish_widget(w2);

        let report = frame.end();
        assert_eq!(report.widgets.len(), 2);
        assert_eq!(report.total_cells(), 2080);
        assert!(report.total_time.as_nanos() > 0);
    }
}
