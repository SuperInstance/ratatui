//! # Render Budget Guardian
//!
//! Frame budget tracking and waste detection for ratatui TUI applications.
//!
//! Most TUI apps silently waste render time: full-screen redraws for a 2-cell change,
//! widgets that allocate `String` on every frame, layouts nested 5+ deep.
//! Guardian makes that waste visible.
//!
//! ## Quick start
//!
//! ```rust
//! use ratatui_guardian::{FrameBudget, RenderProfiler, WasteDetector};
//!
//! let budget = FrameBudget::for_60fps();
//! let mut profiler = RenderProfiler::new(budget);
//!
//! profiler.begin_frame();
//! profiler.begin_widget("StatusBar");
//! // ... your widget renders ...
//! profiler.end_widget(80);
//! profiler.end_frame();
//!
//! let report = profiler.report();
//! println!("{}", report);
//! ```
//!
//! ## Persistence
//!
//! ```rust,ignore
//! profiler.save_json("frame_data.json")?;
//! let loaded = RenderProfiler::load_json("frame_data.json", FrameBudget::for_60fps())?;
//! let comparison = profiler.compare(&loaded)?;
//! ```
//!
//! ## Export formats
//!
//! ```rust,ignore
//! let reporter = Reporter::from_profiler(&profiler);
//! println!("{}", reporter.to_json());
//! println!("{}", reporter.to_prometheus());
//! println!("{}", reporter.to_csv());
//! ```

mod budget;
mod detector;
mod error;
mod profiler;
mod ratatui_adapter;
mod report;
mod reporter;

pub use budget::{BudgetViolation, DetectionConfig, FrameBudget};
pub use detector::{Severity, WasteCategory, WasteDetector, WasteFinding};
pub use error::{GuardianError, Result};
pub use profiler::{PerWidgetStats, RenderProfiler, TrendReport};
pub use ratatui_adapter::{FrameReport, GuardianBuffer, GuardianFrame, WidgetRenderReport};
pub use report::ReportFormatter;
pub use reporter::Reporter;
