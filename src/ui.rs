use std::net::IpAddr;
use std::{collections::HashMap, io, sync::mpsc::Receiver};
use termion::{
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Terminal,
};

pub enum RenderMessage {
    UI(UIUpdate),
    Exit(bool),
}

pub struct UIUpdate {
    pub stats_endpoints: HashMap<String, i64>,
    pub stats_addresses: HashMap<IpAddr, i64>,
    pub avg_rate: u64,
    pub treshold_reached: bool,
}

// Some hell of a type, UIs are not easy
pub fn init_ui(
) -> Result<Terminal<TermionBackend<AlternateScreen<RawTerminal<std::io::Stdout>>>>, io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn draw<B>(
    terminal: &mut Terminal<B>,
    rx_stats: Receiver<RenderMessage>,
    refresh_interval: u64,
    alerting_interval: u64,
    alerting_treshold: u64,
    filename: String,
) -> Result<(), io::Error>
where
    B: Backend,
{
    //    let events = Events::new();
    loop {
        terminal.draw(|f| {
            // Set default content of the UI
            let mut stats_text = vec![];
            let mut stats_endpoints = vec![];
            let mut stats_addresses = vec![];
            let mut border_alert_style = Style::default().fg(Color::White);
            if let Ok(message) = rx_stats.recv() {
                // Modify UI content depending on the content of received message
                match message {
                    RenderMessage::UI(ui_update) => {
                        if ui_update.treshold_reached {
                            border_alert_style = Style::default().fg(Color::Red);
                        }
                        stats_text = vec![
                            Spans::from(vec![
                                Span::styled(
                                    format!("Avg hit rate in last {}s: ", refresh_interval),
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::styled(
                                    format!("{}req/s | ", ui_update.avg_rate),
                                    border_alert_style,
                                ),
                                Span::styled(
                                    "Refresh interval: ",
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::styled(
                                    format!("{}s | ", refresh_interval),
                                    Style::default().fg(Color::White),
                                ),
                                Span::styled(
                                    "Alerting interval: ",
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::styled(
                                    format!("{}s | ", alerting_interval),
                                    Style::default().fg(Color::White),
                                ),
                            ]),
                            Spans::from(vec![
                                Span::styled(
                                    "Alerting treshold: ",
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::styled(
                                    format!("{}req/s | ", alerting_treshold),
                                    border_alert_style,
                                ),
                                Span::styled("Log file: ", Style::default().fg(Color::Cyan)),
                                Span::styled(
                                    format!("{} | ", filename),
                                    Style::default().fg(Color::White),
                                ),
                            ]),
                        ];

                        // Sort items so we see the most popular endpoints in the beginning
                        let mut sorted_endpoints =
                            ui_update.stats_endpoints.iter().collect::<Vec<_>>();
                        let mut sorted_addresses =
                            ui_update.stats_addresses.iter().collect::<Vec<_>>();
                        sorted_endpoints.sort_by(|a, b| b.1.cmp(a.1));
                        sorted_addresses.sort_by(|a, b| b.1.cmp(&a.1));

                        // Build table widgets with statistics
                        for (k, v) in sorted_endpoints.iter() {
                            stats_endpoints.push(Row::StyledData(
                                vec![format!("/{}/*", k), v.to_string()].into_iter(),
                                Style::default().fg(Color::White),
                            ));
                        }
                        for (k, v) in sorted_addresses.iter() {
                            stats_addresses.push(Row::StyledData(
                                vec![k.to_string(), v.to_string()].into_iter(),
                                Style::default().fg(Color::White),
                            ));
                        }
                    }
                    _ => (),
                }
            };

            let create_block = |title| {
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_alert_style)
                    .title(Span::styled(
                        title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
            };

            let paragraph = Paragraph::new(stats_text)
                .block(create_block("All stats"))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            let stats_endpoints = Table::new(
                vec!["Endpoint", "Hits"].into_iter(),
                stats_endpoints.into_iter(),
            )
            .block(create_block("Sorted list of endpoints stats"))
            .widths(&[Constraint::Percentage(80), Constraint::Percentage(20)])
            .column_spacing(1);

            let stats_addresses = Table::new(
                vec!["Address", "Requests"].into_iter(),
                stats_addresses.into_iter(),
            )
            .block(create_block("Sorted list of addresses stats"))
            .widths(&[Constraint::Percentage(80), Constraint::Percentage(20)])
            .column_spacing(1);

            // Divide space horizontally
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(10), Constraint::Percentage(80)].as_ref())
                .split(f.size());

            // Divide bottom space vertically
            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                .split(chunks[1]);
            let left_bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(bottom_chunks[0]);
            f.render_widget(paragraph, chunks[0]);
            f.render_widget(stats_endpoints, left_bottom_chunks[0]);
            f.render_widget(stats_addresses, left_bottom_chunks[1]);
            let block = Block::default().title("Block 4").borders(Borders::ALL);
            f.render_widget(block, bottom_chunks[1]);
        })?;
    }
}
