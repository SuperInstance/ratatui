//! Minimal ratatui-guardian example — 10 lines of instrumentation.
//!
//! Expected output:
//! ```text
//! Frame 1: 0.0ms total (✓, budget: 16ms) 80 cells
//!   MyWidget: 0.0ms (100%) for 80 cells
//!   0 findings
//! ```

use ratatui_guardian::{FrameBudget, RenderProfiler};

fn main() {
    let mut profiler = RenderProfiler::new(FrameBudget::for_60fps());

    profiler.begin_frame();
    profiler.begin_widget("MyWidget");
    // ... your widget render code here ...
    profiler.end_widget(80); // cells written
    profiler.end_frame();

    println!("{}", profiler.report());
}
