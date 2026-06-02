//! Full demo showing all ratatui-guardian features.
//!
//! Expected output:
//! ```text
//! === Frame Report ===
//! Frame 1: X.Xms total (✓, budget: 16ms) 3080 cells
//!   StatusBar: X.Xms (XX%) for 80 cells
//!   Body: X.Xms (XX%) for 2000 cells
//!   Footer: X.Xms (XX%) for 1000 cells
//!   X findings: ...
//!
//! === JSON Export ===
//! {"frame_number": 1, "budget": {...}, "widgets": [...], ...}
//!
//! === Prometheus Export ===
//! guardian_frame_time_us XXX
//! guardian_widget_total_time_us{widget="Body"} XXX
//! ...
//!
//! === CSV Export ===
//! widget,total_time_us,render_count,last_cells,peak_time_us,avg_time_us,full_redraw
//! Body,XXX,1,2000,...
//! ...
//!
//! === Trend Report ===
//! Saved profiler to /tmp/guardian_demo_baseline.json
//! Ran second session...
//! Degraded: 0 widgets
//! Improved: 0 widgets
//! Significant degradation: false
//! ```

use std::thread;
use std::time::Duration;

use ratatui_guardian::{FrameBudget, Reporter, RenderProfiler};

fn main() {
    let budget = FrameBudget::for_60fps();
    let mut profiler = RenderProfiler::new(budget);

    // ── Simulate a frame ──
    profiler.begin_frame();

    profiler.begin_widget("StatusBar");
    thread::sleep(Duration::from_micros(200));
    profiler.end_widget(80);

    profiler.begin_widget("Body");
    thread::sleep(Duration::from_micros(500));
    profiler.end_widget(2000);

    profiler.begin_widget("Footer");
    thread::sleep(Duration::from_micros(100));
    profiler.end_widget(1000);

    profiler.end_frame();

    // ── Human-readable report ──
    println!("=== Frame Report ===");
    println!("{}", profiler.report());

    // ── JSON export ──
    let reporter = Reporter::from_profiler(&profiler);
    println!("=== JSON Export ===");
    println!("{}", reporter.to_json());
    println!();

    // ── Prometheus export ──
    println!("=== Prometheus Export ===");
    println!("{}", reporter.to_prometheus());

    // ── CSV export ──
    println!("=== CSV Export ===");
    println!("{}", reporter.to_csv());

    // ── Persistence + trend analysis ──
    println!("=== Trend Report ===");
    let baseline_path = "/tmp/guardian_demo_baseline.json";
    profiler.save_json(baseline_path).expect("save failed");
    println!("Saved profiler to {baseline_path}");

    // Simulate a second session
    let mut session2 = RenderProfiler::new(FrameBudget::for_60fps());
    session2.begin_frame();
    session2.begin_widget("StatusBar");
    thread::sleep(Duration::from_micros(300)); // slightly slower
    session2.end_widget(80);
    session2.begin_widget("Body");
    thread::sleep(Duration::from_micros(600)); // slightly slower
    session2.end_widget(2000);
    session2.begin_widget("Footer");
    thread::sleep(Duration::from_micros(100));
    session2.end_widget(1000);
    session2.end_frame();

    let trend = session2.compare(&profiler, 0.25).expect("compare failed");
    println!("Ran second session...");
    println!("Degraded: {} widgets", trend.degraded.len());
    println!("Improved: {} widgets", trend.improved.len());
    println!("Significant degradation: {}", trend.significant_degradation);

    for d in &trend.degraded {
        println!(
            "  ↗ {}: {}µs → {}µs ({:+.1}%)",
            d.name, d.previous_avg_us, d.current_avg_us, d.change_percent
        );
    }
    for imp in &trend.improved {
        println!(
            "  ↘ {}: {}µs → {}µs ({:+.1}%)",
            imp.name, imp.previous_avg_us, imp.current_avg_us, imp.change_percent
        );
    }

    // Load baseline back to verify roundtrip
    let loaded =
        RenderProfiler::load_json(baseline_path, FrameBudget::for_60fps()).expect("load failed");
    println!(
        "Loaded baseline: frame {} with {} widgets",
        loaded.last_frame(),
        loaded.widget_stats().len()
    );
}
