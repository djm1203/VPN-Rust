//! Pure rendering for the live dashboard.
//!
//! [`render`] draws the whole cockpit from a borrowed [`Dashboard`] and never
//! mutates state or touches the terminal, so it can be exercised headlessly
//! against ratatui's `TestBackend`. The layout is fully responsive: every
//! region comes from a `Layout` split and every widget clamps itself to the
//! area it is given, so the frame renders without panicking down to a couple of
//! cells.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, LineGauge, List, ListItem, Paragraph, Sparkline},
    Frame,
};

use crate::engine::{ConnectionState, StatsSnapshot};
use crate::tui::app::{format_bytes, format_duration, format_rate, Dashboard};

/// Cyan accent used for headings and highlights.
const ACCENT: Color = Color::Cyan;
/// Border color for the panels.
const BORDER: Color = Color::Rgb(70, 80, 96);
/// Muted color for labels.
const MUTED: Color = Color::Rgb(150, 160, 172);
/// Color for the TX (upload) series.
const TX_COLOR: Color = Color::Rgb(0, 200, 180);
/// Color for the RX (download) series.
const RX_COLOR: Color = Color::Rgb(120, 170, 255);
/// Full scale for the RTT gauge, in milliseconds.
const RTT_SCALE_MS: f64 = 200.0;

/// Draw the entire dashboard for one frame.
pub fn render(frame: &mut Frame, app: &Dashboard) {
    let area = frame.area();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Length(8), // connection + session
            Constraint::Length(7), // throughput sparklines
            Constraint::Min(4),    // logs
            Constraint::Length(1), // help hint
        ])
        .split(area);

    render_title(frame, rows[0], app);
    render_top_row(frame, rows[1], app);
    render_throughput(frame, rows[2], app);
    render_logs(frame, rows[3], app);
    render_help_hint(frame, rows[4]);

    if app.show_help() {
        render_help_overlay(frame, area);
    }
}

/// Title bar: role, uptime, and a colored connection-state badge.
fn render_title(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let role = if app.is_server() { "SERVER" } else { "CLIENT" };
    let state = app.snapshot().state;

    let left = Line::from(vec![
        Span::styled(
            "VPN-Rust",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" — ", Style::default().fg(BORDER)),
        Span::styled(role, Style::default().fg(Color::White)),
        Span::styled(
            format!("  up {}", format_duration(app.uptime())),
            Style::default().fg(MUTED),
        ),
    ]);
    frame.render_widget(Paragraph::new(left), area);

    let badge = Line::from(vec![Span::styled(
        format!(" {} ", state.as_str()),
        Style::default()
            .fg(Color::Black)
            .bg(state_color(state))
            .add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(Paragraph::new(badge).alignment(Alignment::Right), area);
}

/// Top row: connection panel on the left, session panel on the right.
fn render_top_row(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_connection(frame, cols[0], app);
    render_session(frame, cols[1], app);
}

/// Connection panel: state, duration, peer, endpoint, reconnect attempts.
fn render_connection(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let snap = app.snapshot();
    let block = panel(" Connection ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let endpoint_label = if app.is_server() { "Bind" } else { "Server" };
    let lines = vec![
        kv_colored("State", snap.state.as_str(), state_color(snap.state)),
        kv(
            "Uptime",
            &snap
                .connected_for
                .map(format_duration)
                .unwrap_or_else(|| "—".into()),
        ),
        kv("Peer", &opt_addr(snap.peer)),
        kv(endpoint_label, &opt_addr(snap.endpoint)),
        kv("Reconnects", &snap.reconnect_attempts.to_string()),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Session panel: negotiated parameters, RTT gauge, and role.
fn render_session(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let snap = app.snapshot();
    let block = panel(" Session ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 {
        return;
    }

    let (mtu, keepalive) = match snap.negotiated {
        Some(p) => (p.mtu.to_string(), format!("{}s", p.keepalive_secs)),
        None => ("—".to_string(), "—".to_string()),
    };
    let role = if snap.is_server { "server" } else { "client" };

    let sub = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // MTU
            Constraint::Length(1), // keepalive
            Constraint::Length(1), // role
            Constraint::Length(1), // RTT gauge
            Constraint::Min(0),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new(kv("MTU", &mtu)), sub[0]);
    frame.render_widget(Paragraph::new(kv("Keepalive", &keepalive)), sub[1]);
    frame.render_widget(Paragraph::new(kv("Role", role)), sub[2]);
    render_rtt(frame, sub[3], snap);
}

/// RTT row: a `LineGauge` scaled to [`RTT_SCALE_MS`], or a dash when unknown.
fn render_rtt(frame: &mut Frame, area: Rect, snap: &StatsSnapshot) {
    match snap.rtt {
        Some(rtt) => {
            let ms = rtt.as_secs_f64() * 1000.0;
            let ratio = (ms / RTT_SCALE_MS).clamp(0.0, 1.0);
            let gauge = LineGauge::default()
                .filled_style(Style::default().fg(ACCENT))
                .unfilled_style(Style::default().fg(BORDER))
                .label(Span::styled(
                    format!("RTT {ms:>6.1}ms"),
                    Style::default().fg(MUTED),
                ))
                .ratio(ratio);
            frame.render_widget(gauge, area);
        }
        None => {
            frame.render_widget(Paragraph::new(kv("RTT", "—")), area);
        }
    }
}

/// Throughput row: TX and RX sparklines side by side.
fn render_throughput(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let snap = app.snapshot();
    render_series(
        frame,
        cols[0],
        "▲ TX",
        TX_COLOR,
        app.up_history().iter().copied().collect(),
        app.up_rate(),
        snap.bytes_up,
        snap.packets_up,
    );
    render_series(
        frame,
        cols[1],
        "▼ RX",
        RX_COLOR,
        app.down_history().iter().copied().collect(),
        app.down_rate(),
        snap.bytes_down,
        snap.packets_down,
    );
}

/// One throughput panel: a titled sparkline over a cumulative-totals footer.
#[allow(clippy::too_many_arguments)]
fn render_series(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    color: Color,
    data: Vec<u64>,
    rate: u64,
    total_bytes: u64,
    packets: u64,
) {
    let title = format!(" {label}  {} ", format_rate(rate));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 {
        return;
    }

    let sub = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let sparkline = Sparkline::default()
        .style(Style::default().fg(color))
        .data(&data);
    frame.render_widget(sparkline, sub[0]);

    let footer = Line::from(vec![
        Span::styled(format_bytes(total_bytes), Style::default().fg(Color::White)),
        Span::styled(format!("  {packets} pkts"), Style::default().fg(MUTED)),
    ]);
    frame.render_widget(Paragraph::new(footer), sub[1]);
}

/// Scrolling, filtered log panel.
fn render_logs(frame: &mut Frame, area: Rect, app: &Dashboard) {
    let records: Vec<&crate::tui::logbuf::LogRecord> = app.filtered_logs().collect();
    let title = format!(
        " Logs [{}]  {} shown ",
        app.filter().as_str(),
        records.len()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(title, Style::default().fg(ACCENT)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let height = inner.height as usize;
    if height == 0 {
        return;
    }

    // Window over the records: newest at the bottom, `scroll` newest hidden.
    let end = records.len().saturating_sub(app.scroll());
    let start = end.saturating_sub(height);
    let items: Vec<ListItem> = records[start..end]
        .iter()
        .map(|r| {
            let secs = r.at.elapsed().as_secs();
            ListItem::new(Line::from(vec![
                Span::styled(format!("{secs:>4}s "), Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:<5} ", r.level.as_str()),
                    Style::default().fg(level_color(r.level)),
                ),
                Span::styled(
                    format!("{} ", short_target(&r.target)),
                    Style::default().fg(BORDER),
                ),
                Span::styled(r.message.clone(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

/// One-line key-hint bar along the bottom.
fn render_help_hint(frame: &mut Frame, area: Rect) {
    let hint = Line::from(vec![
        Span::styled("q", Style::default().fg(ACCENT)),
        Span::styled(" quit · ", Style::default().fg(MUTED)),
        Span::styled("f", Style::default().fg(ACCENT)),
        Span::styled(" filter · ", Style::default().fg(MUTED)),
        Span::styled("↑/↓", Style::default().fg(ACCENT)),
        Span::styled(" scroll · ", Style::default().fg(MUTED)),
        Span::styled("?", Style::default().fg(ACCENT)),
        Span::styled(" help", Style::default().fg(MUTED)),
    ]);
    frame.render_widget(Paragraph::new(hint), area);
}

/// Centered, bordered help overlay listing all keybindings.
fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(56, 62, area);
    if popup.width == 0 || popup.height == 0 {
        return;
    }
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(Span::styled(
            " Keybindings ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines = vec![
        help_line("q / Esc", "quit the dashboard"),
        help_line("? / h", "toggle this help overlay"),
        help_line("f", "cycle the log filter level"),
        help_line("↑ / k", "scroll logs back (older)"),
        help_line("↓ / j", "scroll logs forward (newer)"),
        help_line("PgUp / PgDn", "scroll logs by a page"),
        help_line("g / Home", "jump to the newest logs"),
        help_line("c", "clear scroll and reset filter"),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

/// A `key  description` row for the help overlay.
fn help_line(keys: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {keys:<12}"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
    ])
}

/// A `label: value` line with a muted label.
fn kv(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(MUTED)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

/// A `label: value` line whose value is colored.
fn kv_colored(label: &str, value: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(MUTED)),
        Span::styled(
            value.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Render an optional socket address, or a dash when absent.
fn opt_addr(addr: Option<std::net::SocketAddr>) -> String {
    addr.map(|a| a.to_string()).unwrap_or_else(|| "—".into())
}

/// Keep only the last path segment of a log target for compact display.
fn short_target(target: &str) -> &str {
    target.rsplit("::").next().unwrap_or(target)
}

/// Badge/label color for a connection state.
fn state_color(state: ConnectionState) -> Color {
    match state {
        ConnectionState::Disconnected => Color::Red,
        ConnectionState::Connecting
        | ConnectionState::Handshaking
        | ConnectionState::Reconnecting => Color::Yellow,
        ConnectionState::Connected => Color::Green,
    }
}

/// Color for a log level tag.
fn level_color(level: crate::tui::logbuf::LogLevel) -> Color {
    use crate::tui::logbuf::LogLevel;
    match level {
        LogLevel::Trace => Color::DarkGray,
        LogLevel::Debug => Color::Gray,
        LogLevel::Info => ACCENT,
        LogLevel::Warn => Color::Yellow,
        LogLevel::Error => Color::Red,
    }
}

/// A `Block` styled as a standard bordered panel with an accented title.
fn panel(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(title, Style::default().fg(ACCENT)))
}

/// A rectangle centered within `area`, sized as a percentage of it.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::control::SessionParams;
    use crate::tui::logbuf::{LogLevel, LogRecord};
    use crossterm::event::KeyCode;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::time::Instant;

    fn connected_snapshot() -> StatsSnapshot {
        StatsSnapshot {
            state: ConnectionState::Connected,
            bytes_up: 4096,
            bytes_down: 8192,
            packets_up: 12,
            packets_down: 20,
            rtt: Some(std::time::Duration::from_millis(42)),
            connected_for: Some(std::time::Duration::from_secs(75)),
            reconnect_attempts: 1,
            peer: Some("203.0.113.5:5555".parse().unwrap()),
            negotiated: Some(SessionParams {
                mtu: 1380,
                keepalive_secs: 15,
            }),
            is_server: false,
            endpoint: Some("0.0.0.0:5555".parse().unwrap()),
        }
    }

    fn disconnected_snapshot() -> StatsSnapshot {
        StatsSnapshot {
            state: ConnectionState::Disconnected,
            bytes_up: 4096,
            bytes_down: 8192,
            packets_up: 12,
            packets_down: 20,
            rtt: None,
            connected_for: None,
            reconnect_attempts: 1,
            peer: None,
            negotiated: None,
            is_server: false,
            endpoint: Some("0.0.0.0:5555".parse().unwrap()),
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn renders_expected_content_at_full_size() {
        let mut app = Dashboard::new(false);
        app.on_tick(
            disconnected_snapshot(),
            vec![LogRecord {
                at: Instant::now(),
                level: LogLevel::Info,
                target: "vpn_rust::engine".into(),
                message: "listening".into(),
            }],
        );
        app.on_tick(connected_snapshot(), vec![]);

        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("VPN-Rust"), "title missing");
        assert!(text.contains("Connected"), "state missing");
        assert!(text.contains("TX"), "TX series missing");
        assert!(text.contains("RX"), "RX series missing");
        assert!(text.contains("1380"), "negotiated MTU missing");
    }

    #[test]
    fn renders_without_panic_on_tiny_terminal() {
        let mut app = Dashboard::new(true);
        app.on_tick(connected_snapshot(), vec![]);
        let mut terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();
    }

    #[test]
    fn renders_help_overlay() {
        let mut app = Dashboard::new(false);
        app.on_tick(connected_snapshot(), vec![]);
        app.on_key(KeyCode::Char('?'));

        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let text = buffer_text(&terminal);
        assert!(text.contains("Keybindings"), "help overlay missing");
    }
}
