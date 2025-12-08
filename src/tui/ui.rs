//! UI rendering for the TUI.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::app::{format_bytes, App};

/// Render the entire UI.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title bar
            Constraint::Length(7),  // Status panel
            Constraint::Length(9),  // Statistics panel
            Constraint::Min(10),    // Log panel
            Constraint::Length(1),  // Help bar
        ])
        .split(frame.size());

    render_title(frame, chunks[0], app);
    render_status(frame, chunks[1], app);
    render_stats(frame, chunks[2], app);
    render_logs(frame, chunks[3], app);
    render_help(frame, chunks[4]);
}

/// Render the title bar.
fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    let mode = if app.is_server { "SERVER" } else { "CLIENT" };
    let title = format!(" VPN-Rust {} ", mode);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let uptime = format!("Uptime: {}", app.uptime());
    let content = Paragraph::new(uptime)
        .style(Style::default().fg(Color::Gray))
        .block(block);

    frame.render_widget(content, area);
}

/// Render the connection status panel.
fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .title(Span::styled(
            " Connection ",
            Style::default().fg(Color::Blue),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let status_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(inner);

    // Status line
    let status_color = app.state.color();
    let status_line = Line::from(vec![
        Span::styled("Status: ", Style::default().fg(Color::Gray)),
        Span::styled(
            app.state.as_str(),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(status_line), status_chunks[0]);

    // Duration line
    let duration_line = Line::from(vec![
        Span::styled("Duration: ", Style::default().fg(Color::Gray)),
        Span::styled(
            app.connection_duration(),
            Style::default().fg(Color::White),
        ),
    ]);
    frame.render_widget(Paragraph::new(duration_line), status_chunks[1]);

    // Peer info line
    let peer_label = if app.is_server { "Listening: " } else { "Server: " };
    let peer_line = Line::from(vec![
        Span::styled(peer_label, Style::default().fg(Color::Gray)),
        Span::styled(&app.peer_info, Style::default().fg(Color::White)),
    ]);
    frame.render_widget(Paragraph::new(peer_line), status_chunks[2]);

    // Client count (server mode only)
    if app.is_server {
        let client_line = Line::from(vec![
            Span::styled("Clients: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.client_count.to_string(),
                Style::default().fg(Color::Cyan),
            ),
        ]);
        frame.render_widget(Paragraph::new(client_line), status_chunks[3]);
    }
}

/// Render the statistics panel.
fn render_stats(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            " Traffic Statistics ",
            Style::default().fg(Color::Magenta),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // Left side: Sent
    let sent_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(stats_chunks[0]);

    let bytes_sent = app.stats.get_bytes_sent();
    let packets_sent = app.stats.get_packets_sent();

    let sent_title = Paragraph::new(Line::from(vec![Span::styled(
        "↑ SENT",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(sent_title, sent_chunks[0]);

    let sent_bytes = Paragraph::new(Line::from(vec![
        Span::styled("Bytes: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(bytes_sent),
            Style::default().fg(Color::White),
        ),
    ]));
    frame.render_widget(sent_bytes, sent_chunks[2]);

    let sent_packets = Paragraph::new(Line::from(vec![
        Span::styled("Packets: ", Style::default().fg(Color::Gray)),
        Span::styled(
            packets_sent.to_string(),
            Style::default().fg(Color::White),
        ),
    ]));
    frame.render_widget(sent_packets, sent_chunks[3]);

    // Right side: Received
    let recv_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(stats_chunks[1]);

    let bytes_recv = app.stats.get_bytes_received();
    let packets_recv = app.stats.get_packets_received();

    let recv_title = Paragraph::new(Line::from(vec![Span::styled(
        "↓ RECEIVED",
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(recv_title, recv_chunks[0]);

    let recv_bytes = Paragraph::new(Line::from(vec![
        Span::styled("Bytes: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_bytes(bytes_recv),
            Style::default().fg(Color::White),
        ),
    ]));
    frame.render_widget(recv_bytes, recv_chunks[2]);

    let recv_packets = Paragraph::new(Line::from(vec![
        Span::styled("Packets: ", Style::default().fg(Color::Gray)),
        Span::styled(
            packets_recv.to_string(),
            Style::default().fg(Color::White),
        ),
    ]));
    frame.render_widget(recv_packets, recv_chunks[3]);
}

/// Render the log panel.
fn render_logs(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(" Logs ", Style::default().fg(Color::Yellow)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let log_items: Vec<ListItem> = app
        .logs
        .iter()
        .rev()
        .take(inner.height as usize)
        .map(|entry| {
            let elapsed = entry.timestamp.elapsed();
            let time_str = format!("{:>5}s", elapsed.as_secs());

            let line = Line::from(vec![
                Span::styled(
                    format!("[{}] ", time_str),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:<5} ", entry.level.as_str()),
                    Style::default().fg(entry.level.color()),
                ),
                Span::raw(&entry.message),
            ]);

            ListItem::new(line)
        })
        .collect();

    let logs_list = List::new(log_items).style(Style::default().fg(Color::White));

    frame.render_widget(logs_list, inner);
}

/// Render the help bar.
fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::styled(" q ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(" Quit  "),
        Span::styled(" c ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(" Clear logs  "),
        Span::styled(" r ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(" Reconnect  "),
    ]);

    let help = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));

    frame.render_widget(help, area);
}
