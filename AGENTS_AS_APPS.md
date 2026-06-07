# Agents as Applications: open-tui

> The agent doesn't have a dashboard. The agent *is* the dashboard.

## The Shift

Most monitoring tools observe systems from the outside. **open-tui** turns Ratatui into the agent's native body. The terminal isn't a window into the agent — it's the agent's skin. Every frame the TUI renders is a direct projection of the agent's internal state: `fleet-warden` health data becomes ASCII vital signs, `conservation-law` energy budgets become animated gauges, `spectral-fleet` eigenvalues become a live bar chart. The user isn't watching a monitoring system. They're looking directly at the agent's nervous system.

The agent doesn't output to a dashboard. The agent's cognition *is* the dashboard. When the agent rebalances a fleet, the TUI doesn't receive a notification — it *is* the rebalancing, rendered as shifting color blocks in real time.

## Live Agent Vitals

### Fleet-Warden Health Monitor

```rust
use ratatui::widgets::{Block, Borders, Gauge, Sparkline};
use ratatui::style::{Color, Style};
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::Frame;

/// The agent's heartbeat rendered as ASCII.
fn render_agent_health(frame: &mut Frame, report: &ScanReport, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Disk gauge
            Constraint::Length(3), // Cache gauge
            Constraint::Length(3), // Sessions gauge
        ])
        .split(area);

    let total = report.total_cleanable() as f64;
    let max = 100_000_000_000.0; // 100 GB scale

    let disk_gauge = Gauge::default()
        .block(Block::default().title("Agent Disk Load").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio((total / max).min(1.0));
    frame.render_widget(disk_gauge, chunks[0]);

    let cache_data: Vec<u64> = vec![
        report.pip_cache_size,
        report.npm_cache_size,
        report.huggingface_size,
    ];
    let spark = Sparkline::default()
        .block(Block::default().title("Cache Pressure").borders(Borders::ALL))
        .data(&cache_data)
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(spark, chunks[1]);
}
```

### Conservation-Law Energy Budget Gauges

```rust
use ratatui::widgets::Gauge;
use conservation_law::lagrangian::{total_energy, MechanicalLagrangian, AgentState};

/// The agent's cognitive energy budget as an ASCII gauge.
/// When energy drifts, the agent is working too hard and needs to rest.
fn render_energy_budget(frame: &mut Frame, trajectory: &[AgentState<f64, 2>], area: ratatui::layout::Rect) {
    if trajectory.len() < 2 {
        return;
    }

    let potential = |q: &[f64; 2]| 0.5 * (q[0] * q[0] + q[1] * q[1]);
    let lagrangian = MechanicalLagrangian { mass: 1.0, potential_fn: potential };

    let e0 = total_energy(&lagrangian, &trajectory[0]);
    let e_final = total_energy(&lagrangian, trajectory.last().unwrap());
    let drift = (e_final - e0).abs();

    // Normalize: 0% = perfect conservation, 100% = catastrophic drift
    let drift_ratio = (drift / e0).min(1.0);
    let (label, color) = if drift_ratio < 0.001 {
        ("Agent Calm", Color::Green)
    } else if drift_ratio < 0.01 {
        ("Agent Working", Color::Yellow)
    } else {
        ("Agent Overheated", Color::Red)
    };

    let gauge = Gauge::default()
        .block(Block::default().title("Cognitive Energy Budget").borders(Borders::ALL))
        .gauge_style(Style::default().fg(color))
        .ratio(drift_ratio)
        .label(label);
    frame.render_widget(gauge, area);
}
```

### Spectral-Fleet Eigenvalue Visualization

```rust
use ratatui::widgets::{BarChart, Block, Borders};
use spectral_fleet::power_iteration::Eigenpair;

/// The agent's principal modes of thought, rendered as bars.
fn render_eigenvalue_spectrum(frame: &mut Frame, pairs: &[Eigenpair<f64>], area: ratatui::layout::Rect) {
    let data: Vec<(&str, u64)> = pairs.iter().enumerate()
        .map(|(i, pair)| {
            let label = format!("λ{}", i);
            let val = (pair.value.abs() * 10.0) as u64;
            // Leak the label string so BarChart can borrow it
            let leaked: &'static str = Box::leak(label.into_boxed_str());
            (leaked, val)
        })
        .collect();

    let chart = BarChart::default()
        .block(Block::default().title("Agent Thought Modes (Eigenvalues)").borders(Borders::ALL))
        .data(&data)
        .bar_width(7)
        .bar_style(Style::default().fg(Color::Magenta))
        .value_style(Style::default().fg(Color::White));
    frame.render_widget(chart, area);
}
```

## Agent Event Loop as TUI Frame Loop

```rust
use ratatui::DefaultTerminal;
use crossterm::event::{self, Event, KeyCode};
use std::time::{Duration, Instant};

/// The agent's main loop IS the TUI frame loop.
fn run_agent_dashboard() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    // Agent state
    let mut scan_report = fleet_warden::scanner::full_scan().unwrap_or_default();
    let mut trajectory: Vec<AgentState<f64, 2>> = vec![AgentState::new([1.0, 0.0], [0.0, 1.0])];

    loop {
        terminal.draw(|frame| {
            let main = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main);
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[0]);

            render_agent_health(frame, &scan_report, left_chunks[0]);
            render_energy_budget(frame, &trajectory, left_chunks[1]);
            // render_eigenvalue_spectrum(frame, &eigenpairs, chunks[1]);
        })?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('r') => {
                        // Agent refreshes its own perception
                        scan_report = fleet_warden::scanner::full_scan().unwrap_or_default();
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            // Agent updates its symplectic trajectory every tick
            let integrator = SymplecticIntegrator::new(0.001).unwrap();
            let potential = |q: &[f64; 2]| 0.5 * (q[0] * q[0] + q[1] * q[1]);
            if let Ok(next) = integrator.step(1.0, &potential, trajectory.last().unwrap()) {
                trajectory.push(next);
                if trajectory.len() > 1000 {
                    trajectory.remove(0);
                }
            }
            last_tick = Instant::now();
        }
    }

    ratatui::restore();
    Ok(())
}
```

## What This Enables

**Terminal-native agents.** The agent doesn't need a browser, a GPU, or a window manager. It lives in the terminal, where every sysadmin, SRE, and DevOps engineer already works. The agent's face is ASCII, but its cognition is full-spectrum mathematics.

**Low-bandwidth telepresence.** SSH into a server and the agent greets you with its health gauge, energy budget, and thought-mode spectrum. No X11 forwarding. No web server. Just pure terminal agent embodiment.

**Keyboard-driven agent interaction.** Every Ratatui key event is an agent command. Press `r` and the agent rescanes the fleet. Press `c` and the agent runs a symplectic integration step. Press `q` and the agent dies gracefully, persisting its state.

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                    Terminal Screen                     │
│  ┌──────────────┐  ┌────────────────────────────────┐ │
│  │ Disk Health  │  │                                │ │
│  │ (fleet-warden)│  │  Eigenvalue Spectrum           │ │
│  │ [=====>    ] │  │  ▓▓▓▓▓▓▓ λ0                   │ │
│  └──────────────┘  │  ▓▓▓▓▓ λ1                      │ │
│  ┌──────────────┐  │  ▓▓▓ λ2                        │ │
│  │ Energy Budget│  │  ▓ λ3                          │ │
│  │ [==>       ] │  │                                │ │
│  │ Agent Calm   │  │  (spectral-fleet)              │ │
│  └──────────────┘  └────────────────────────────────┘ │
│  (conservation-law)                                    │
└────────────────────────────────────────────────────────┘
```

The TUI has no "update loop" separate from the agent. The 100ms tick *is* the agent's consciousness cycle. Every frame refresh is the agent thinking out loud.

## Next Steps

1. **Braille sparklines for eigenvectors** — Use ratatui's Braille canvas to render high-resolution eigenvector trajectories.
2. **Color-coded agent mood** — Map `conservation-law` energy drift to a terminal color theme that changes when the agent is stressed.
3. **Agent log as scrollback** — Every thought the agent has is appended to a ratatui `Paragraph` widget with timestamp and severity.
4. **Modal agent cognition** — Vim-style modal interface: Normal mode watches, Insert mode lets the user type agent commands, Visual mode selects fleet nodes for batch operations.
5. **TUI-to-WASM bridge** — Compile the agent's Rust core to WASM and embed it in a Tauri desktop app when the user wants to leave the terminal.
