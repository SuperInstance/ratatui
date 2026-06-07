//! # Agent Fleet Dashboard
//!
//! A live TUI dashboard demonstrating agents-as-applications concepts from
//! the SuperInstance ecosystem. Renders:
//!
//! - **Fleet health** — agent status grid (fleet-warden)
//! - **Energy budgets** — ASCII bar charts (conservation-law)
//! - **Spectral rankings** — eigenvalue-ordered agent leaderboard (spectral-fleet)
//! - **T-minus countdown** — mission timers
//! - **Entropy accounting** — thermodynamic budget tracking
//!
//! Run: `cargo run -p agent-dashboard`

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};

use crossterm::{
    event::{self, Event, KeyCode, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::io;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Domain models (mirroring SuperInstance crate concepts)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Agent {
    name: String,
    status: AgentStatus,
    energy_budget: f64,
    energy_used: f64,
    eigenvalue: f64,
    entropy_produced: f64,
    position: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AgentStatus {
    Healthy,
    Degraded,
    Critical,
    Offline,
}

impl AgentStatus {
    fn color(self) -> Color {
        match self {
            Self::Healthy => Color::Green,
            Self::Degraded => Color::Yellow,
            Self::Critical => Color::Red,
            Self::Offline => Color::DarkGray,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Healthy => "OK",
            Self::Degraded => "DEG",
            Self::Critical => "CRIT",
            Self::Offline => "OFF",
        }
    }
}

#[derive(Debug, Clone)]
struct FleetState {
    agents: Vec<Agent>,
    mission_start: Instant,
    mission_duration: Duration,
    total_entropy: f64,
    entropy_budget: f64,
    tick: u64,
}

impl FleetState {
    fn new() -> Self {
        Self {
            agents: vec![
                Agent {
                    name: "alpha-7".into(),
                    status: AgentStatus::Healthy,
                    energy_budget: 100.0,
                    energy_used: 34.2,
                    eigenvalue: 0.97,
                    entropy_produced: 12.3,
                    position: (0.1, 0.3),
                },
                Agent {
                    name: "beta-3".into(),
                    status: AgentStatus::Healthy,
                    energy_budget: 100.0,
                    energy_used: 58.1,
                    eigenvalue: 0.82,
                    entropy_produced: 28.7,
                    position: (0.5, 0.2),
                },
                Agent {
                    name: "gamma-1".into(),
                    status: AgentStatus::Degraded,
                    energy_budget: 80.0,
                    energy_used: 72.4,
                    eigenvalue: 0.61,
                    entropy_produced: 41.2,
                    position: (0.8, 0.7),
                },
                Agent {
                    name: "delta-9".into(),
                    status: AgentStatus::Critical,
                    energy_budget: 60.0,
                    energy_used: 59.8,
                    eigenvalue: 0.23,
                    entropy_produced: 55.0,
                    position: (0.3, 0.9),
                },
                Agent {
                    name: "epsilon-2".into(),
                    status: AgentStatus::Offline,
                    energy_budget: 100.0,
                    energy_used: 0.0,
                    eigenvalue: 0.0,
                    entropy_produced: 0.0,
                    position: (0.0, 0.0),
                },
                Agent {
                    name: "zeta-5".into(),
                    status: AgentStatus::Healthy,
                    energy_budget: 120.0,
                    energy_used: 22.0,
                    eigenvalue: 0.94,
                    entropy_produced: 8.1,
                    position: (0.6, 0.4),
                },
            ],
            mission_start: Instant::now() - Duration::from_secs(1847),
            mission_duration: Duration::from_secs(3600),
            total_entropy: 145.3,
            entropy_budget: 200.0,
            tick: 0,
        }
    }

    fn tick(&mut self) {
        self.tick += 1;
        // Simulate energy consumption and entropy production
        for agent in &mut self.agents {
            if agent.status != AgentStatus::Offline {
                agent.energy_used += 0.1;
                agent.entropy_produced += 0.05;
                agent.eigenvalue = (agent.eigenvalue * 100.0 - 0.01).max(0.0) / 100.0;
            }
        }
        self.total_entropy = self.agents.iter().map(|a| a.entropy_produced).sum();
        // Occasionally degrade
        if self.tick % 100 == 0 {
            if let Some(agent) = self.agents.iter_mut().find(|a| a.status == AgentStatus::Healthy) {
                agent.status = AgentStatus::Degraded;
            }
        }
    }

    fn time_remaining(&self) -> Duration {
        self.mission_duration.saturating_sub(self.mission_start.elapsed())
    }

    fn sorted_by_eigenvalue(&self) -> Vec<&Agent> {
        let mut sorted: Vec<&Agent> = self.agents.iter().collect();
        sorted.sort_by(|a, b| b.eigenvalue.partial_cmp(&a.eigenvalue).unwrap_or(std::cmp::Ordering::Equal));
        sorted
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render_dashboard(frame: &mut Frame, state: &FleetState) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title bar
            Constraint::Min(0),    // Main body
            Constraint::Length(3),  // Status bar
        ])
        .split(frame.area());

    // Title bar
    let title = Paragraph::new(Line::from(vec![
        Span::styled("◆ ", Style::default().fg(Color::Cyan)),
        Span::styled("AGENT FLEET DASHBOARD", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" ── SuperInstance Fleet Monitor", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("  [tick {}]", state.tick), Style::default().fg(Color::Yellow)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).style(Style::default().fg(Color::Cyan)));
    frame.render_widget(title, outer[0]);

    // Main body — split into left and right columns
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    // LEFT COLUMN
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Fleet health grid
            Constraint::Min(8),     // Energy budgets
        ])
        .split(body[0]);

    render_fleet_health(frame, left[0], state);
    render_energy_budgets(frame, left[1], state);

    // RIGHT COLUMN
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Spectral rankings
            Constraint::Length(5),  // T-minus countdown
            Constraint::Min(5),    // Entropy accounting
        ])
        .split(body[1]);

    render_spectral_rankings(frame, right[0], state);
    render_countdown(frame, right[1], state);
    render_entropy_accounting(frame, right[2], state);

    // Status bar
    let remaining = state.time_remaining();
    let secs = remaining.as_secs();
    let status = Paragraph::new(Line::from(vec![
        Span::styled(" FLEET: ", Style::default().fg(Color::Black).bg(Color::Green)),
        Span::raw(" "),
        Span::styled(format!("{} agents online", state.agents.iter().filter(|a| a.status != AgentStatus::Offline).count()),
        Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(" ENTROPY: ", Style::default().fg(Color::Black).bg(Color::Magenta)),
        Span::raw(" "),
        Span::styled(format!("{:.1}/{:.0} units", state.total_entropy, state.entropy_budget), Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(" T-MINUS: ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(" "),
        Span::styled(format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60), Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(" [Q]uit", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(status, outer[2]);
}

fn render_fleet_health(frame: &mut Frame, area: Rect, state: &FleetState) {
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(" Fleet Health (fleet-warden)", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for agent in &state.agents {
        let status_indicator = Span::styled(
            format!(" ● {} ", agent.status.label()),
            Style::default().fg(agent.status.color()),
        );
        let name = Span::styled(
            format!("{:<12}", agent.name),
            Style::default().fg(Color::White),
        );
        let pos = Span::styled(
            format!(" pos({:.1},{:.1})", agent.position.0, agent.position.1),
            Style::default().fg(Color::DarkGray),
        );
        lines.push(Line::from(vec![status_indicator, name, pos]));
    }

    let block = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(block, area);
}

fn render_energy_budgets(frame: &mut Frame, area: Rect, state: &FleetState) {
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(" Energy Budgets (conservation-law)", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for agent in &state.agents {
        if agent.status == AgentStatus::Offline {
            lines.push(Line::from(Span::styled(
                format!(" {} {:>12}  [OFFLINE]", agent.name, ""),
                Style::default().fg(Color::DarkGray),
            )));
            continue;
        }

        let ratio = (agent.energy_used / agent.energy_budget).min(1.0);
        let bar_width = 20;
        let filled = (ratio * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let bar_color = if ratio < 0.5 {
            Color::Green
        } else if ratio < 0.8 {
            Color::Yellow
        } else {
            Color::Red
        };

        let bar = format!(
            "{}{}",
            "█".repeat(filled),
            "░".repeat(empty),
        );

        lines.push(Line::from(vec![
            Span::styled(format!(" {:<10}", agent.name), Style::default().fg(Color::White)),
            Span::styled(bar, Style::default().fg(bar_color)),
            Span::styled(format!(" {:.0}%", ratio * 100.0), Style::default().fg(bar_color)),
        ]));
    }

    // Total fleet energy as a gauge-like display
    let total_used: f64 = state.agents.iter().map(|a| a.energy_used).sum();
    let total_budget: f64 = state.agents.iter().map(|a| a.energy_budget).sum();
    let total_ratio = (total_used / total_budget).min(1.0);

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" FLEET TOTAL ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{:.1}/{:.0} TJ ", total_used, total_budget),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    // Use a Gauge widget for the total
    let gauge_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Line::from(Span::styled(
            " Conservation Law Compliance ",
            Style::default().fg(Color::DarkGray),
        )));
    let paragraph = Paragraph::new(lines).block(gauge_block);
    frame.render_widget(paragraph, area);
}

fn render_spectral_rankings(frame: &mut Frame, area: Rect, state: &FleetState) {
    let ranked = state.sorted_by_eigenvalue();

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(" Spectral Rankings (spectral-fleet)", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for (i, agent) in ranked.iter().enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };

        let bar_width = 15;
        let filled = (agent.eigenvalue * bar_width as f64) as usize;
        let bar = format!(
            "{}{}",
            "▓".repeat(filled),
            "░".repeat(bar_width - filled),
        );

        let rank_color = if agent.eigenvalue > 0.8 {
            Color::Green
        } else if agent.eigenvalue > 0.5 {
            Color::Yellow
        } else if agent.eigenvalue > 0.0 {
            Color::Red
        } else {
            Color::DarkGray
        };

        lines.push(Line::from(vec![
            Span::raw(format!(" {} ", medal)),
            Span::styled(format!("{:<10}", agent.name), Style::default().fg(Color::White)),
            Span::styled(bar, Style::default().fg(rank_color)),
            Span::styled(format!(" {:.3}", agent.eigenvalue), Style::default().fg(rank_color)),
        ]));
    }

    let block = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(block, area);
}

fn render_countdown(frame: &mut Frame, area: Rect, state: &FleetState) {
    let remaining = state.time_remaining();
    let secs = remaining.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    let ratio = remaining.as_secs_f64() / state.mission_duration.as_secs_f64();

    let countdown_text = format!(" T─ {:02}:{:02}:{:02} ", hours, minutes, seconds);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(" Mission Clock", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                countdown_text,
                Style::default().fg(if ratio < 0.1 { Color::Red } else if ratio < 0.3 { Color::Yellow } else { Color::White }).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({:.0}% remaining)", ratio * 100.0),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let block = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(block, area);
}

fn render_entropy_accounting(frame: &mut Frame, area: Rect, state: &FleetState) {
    let entropy_ratio = (state.total_entropy / state.entropy_budget).min(1.0);
    let budget_remaining = (state.entropy_budget - state.total_entropy).max(0.0);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            " Entropy Accounting (conservation-law)",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Produced: ", Style::default().fg(Color::White)),
            Span::styled(format!("{:.1} units", state.total_entropy), Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::styled(" Budget:   ", Style::default().fg(Color::White)),
            Span::styled(format!("{:.1} units", state.entropy_budget), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled(" Remaining:", Style::default().fg(Color::White)),
            Span::styled(
                format!(" {:.1} units", budget_remaining),
                Style::default().fg(if budget_remaining < 30.0 { Color::Red } else { Color::Yellow }),
            ),
        ]),
    ];

    // Entropy bar
    let bar_width = 30;
    let filled = (entropy_ratio * bar_width as f64) as usize;
    lines.push(Line::from(""));

    let entropy_color = if entropy_ratio < 0.5 {
        Color::Green
    } else if entropy_ratio < 0.8 {
        Color::Yellow
    } else {
        Color::Red
    };

    lines.push(Line::from(vec![
        Span::styled(" [", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "█".repeat(filled),
            Style::default().fg(entropy_color),
        ),
        Span::styled(
            "░".repeat(bar_width - filled),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("]", Style::default().fg(Color::DarkGray)),
        Span::styled(format!(" {:.0}%", entropy_ratio * 100.0), Style::default().fg(entropy_color)),
    ]));

    // Per-agent entropy breakdown
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Per-agent breakdown:",
        Style::default().fg(Color::DarkGray),
    )));
    for agent in &state.agents {
        if agent.entropy_produced > 0.0 {
            lines.push(Line::from(vec![
                Span::styled(format!("   {:<10}", agent.name), Style::default().fg(Color::White)),
                Span::styled(format!("{:.1} units", agent.entropy_produced), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    let block = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(block, area);
}

// ---------------------------------------------------------------------------
// Terminal setup / main loop
// ---------------------------------------------------------------------------

fn run() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // State
    let mut fleet = FleetState::new();
    let tick_rate = Duration::from_millis(500);
    let mut last_tick = Instant::now();

    // Main loop
    loop {
        // Draw
        terminal.draw(|f| render_dashboard(f, &fleet))?;

        // Handle events
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }

        // Tick
        if last_tick.elapsed() >= tick_rate {
            fleet.tick();
            last_tick = Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn main() -> io::Result<()> {
    run()
}
