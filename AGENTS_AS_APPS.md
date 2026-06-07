# Agents as Applications: The Agent IS the Dashboard

## Vision

In the agents-as-applications paradigm, the agent doesn't *generate* dashboard code — it **produces the dashboard output directly**. Open-tui becomes the rendering surface through which agents manifest as live, interactive terminal applications.

## The Agent-Dashboard Identity

Traditional approach:
```
Agent → generates Python/JS → dashboard library → renders UI
```

Agents-as-applications:
```
Agent → IS the dashboard → renders through open-tui directly
```

The agent dashboard example (`examples/agent-dashboard/`) demonstrates this with five SuperInstance crate concepts:

### Fleet Health (fleet-warden)
- Real-time agent status grid
- Color-coded health indicators (Healthy → Degraded → Critical → Offline)
- Position tracking in room topology space

### Energy Budgets (conservation-law)
- ASCII bar charts showing energy consumption per agent
- Conservation law compliance: total used vs. total budget
- Visual warning thresholds (green → yellow → red)

### Spectral Rankings (spectral-fleet)
- Eigenvalue-ordered agent leaderboard
- Bar representation of each agent's spectral score
- Medal system for top performers

### Mission Clock
- T-minus countdown timer
- Percentage remaining indicator
- Urgency-based color coding

### Entropy Accounting
- Total entropy produced vs. budget
- Per-agent entropy breakdown
- Thermodynamic budget visualization

## How It Works

The `FleetState` struct is the agent's *internal state*. The render functions are the agent's *outputs*. The terminal is the agent's *body*.

```
┌─────────────────────────────────────┐
│         FleetState (Agent)          │
│                                     │
│  ┌──────┐ ┌──────┐ ┌──────┐       │
│  │alpha │ │beta  │ │gamma │ ...    │
│  └──┬───┘ └──┬───┘ └──┬───┘       │
│     │        │        │            │
│     ▼        ▼        ▼            │
│  tick() → state transitions        │
│     │                              │
│     ▼                              │
│  render_dashboard()                │
│     │                              │
│     ▼                              │
│  Terminal output (the agent's body)│
└─────────────────────────────────────┘
```

## Extending

To add a new panel (e.g., intention-field visualization):

1. Add the data to `Agent` or `FleetState`
2. Create a `render_intention_field()` function
3. Add a layout slot in `render_dashboard()`
4. Update `tick()` to simulate intention dynamics

The agent becomes the application — no code generation step, no build step, just direct manifestation.
