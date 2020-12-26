use chrono::Utc;
use std::{collections::HashMap, io, net::IpAddr, sync::mpsc::Receiver, vec::IntoIter};
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
    Exit,
}

pub struct UIUpdate {
    pub stats_endpoints: HashMap<String, u64>,
    pub stats_addresses: HashMap<IpAddr, u64>,
    pub stats_http_codes: HashMap<u16, HashMap<String, u64>>,
    pub avg_rate: u64,
    pub treshold_reached: bool,
    pub log_samples: Vec<String>,
    pub avg_within_alert_interval: u64,
}

type DynTables<'a> = Vec<Table<'a, IntoIter<String>, IntoIter<Row<IntoIter<String>>>>>;
type Rows = Vec<Row<IntoIter<String>>>;
impl Default for UIUpdate {
    fn default() -> Self {
        UIUpdate {
            stats_endpoints: HashMap::new(),
            stats_addresses: HashMap::new(),
            stats_http_codes: HashMap::new(),
            avg_rate: 0,
            treshold_reached: false,
            log_samples: vec![
                String::from("Waiting for data from file-reading thread"),
                String::from("Waiting for data from file-reading thread"),
                String::from("Waiting for data from file-reading thread"),
            ],
            avg_within_alert_interval: 0,
        }
    }
}

struct AlertState {
    change_time: String,
    treshold_reached: bool,
    avg_within_alert_interval: u64,
}

impl AlertState {
    fn reverse(&mut self) {
        self.treshold_reached = !self.treshold_reached;
        self.change_time = Utc::now().format("%H:%M:%S").to_string();
    }

    fn to_spans(&self) -> Vec<Spans> {
        if self.treshold_reached {
            vec![
                Spans::from(Span::styled(
                    String::from("State: Firing"),
                    Style::default().fg(Color::Red),
                )),
                Spans::from(Span::styled(
                    format!("Firing since: {}", self.change_time),
                    Style::default().fg(Color::Red),
                )),
                Spans::from(Span::styled(
                    format!("Avg rate: {}", self.avg_within_alert_interval),
                    Style::default().fg(Color::Red),
                )),
            ]
        } else {
            vec![
                Spans::from(Span::styled(
                    String::from("State: Resolved"),
                    Style::default().fg(Color::Green),
                )),
                Spans::from(Span::styled(
                    format!("\nResolved at: {}", self.change_time),
                    Style::default().fg(Color::Green),
                )),
            ]
        }
    }
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
    // State for the alert block, have to be in outer scope to persist
    let mut alert_state = AlertState {
        treshold_reached: false,
        change_time: String::from(""),
        avg_within_alert_interval: 0,
    };
    let value_style = Style::default().fg(Color::White);
    let key_style = Style::default().fg(Color::Cyan);

    loop {
        let mut exit_signal = false;
        terminal.draw(|f| {
            // Set default content of the UI
            let mut stats_text = vec![];
            let mut stats_endpoints = vec![];
            let mut stats_addresses = vec![];
            let mut stats_http_codes: Vec<(u16, Rows)> = vec![];
            let mut log_samples = vec![];
            let mut alert_text = vec![Spans::from(vec![Span::styled(
                "Alert didn't happen to fire since start",
                Style::default().fg(Color::White),
            )])];
            let mut alert_style = Style::default().fg(Color::White);
            if let Ok(message) = rx_stats.recv() {
                // Modify UI content depending on the content of received message
                match message {
                    RenderMessage::UI(ui_update) => {
                        // Alerting values and styling
                        if ui_update.treshold_reached != alert_state.treshold_reached {
                            alert_state.reverse();
                        }
                        alert_state.avg_within_alert_interval = ui_update.avg_within_alert_interval;
                        if alert_state.change_time != *"" {
                            alert_text = alert_state.to_spans();
                            alert_style = if alert_state.treshold_reached {
                                Style::default().fg(Color::Red)
                            } else {
                                Style::default().fg(Color::Green)
                            }
                        };

                        // General statistics and settings
                        stats_text = vec![
                            Spans::from(vec![
                                Span::styled(
                                    format!("Avg hit rate in last {}s: ", refresh_interval),
                                    key_style,
                                ),
                                Span::styled(
                                    format!("{}req/s | ", ui_update.avg_rate),
                                    alert_style,
                                ),
                                Span::styled("Refresh interval: ", key_style),
                                Span::styled(format!("{}s | ", refresh_interval), value_style),
                                Span::styled("Alerting interval: ", key_style),
                                Span::styled(format!("{}s | ", alerting_interval), value_style),
                            ]),
                            Spans::from(vec![
                                Span::styled("Alerting treshold: ", key_style),
                                Span::styled(format!("{}req/s | ", alerting_treshold), alert_style),
                                Span::styled("Log file: ", key_style),
                                Span::styled(format!("{} | ", filename), value_style),
                            ]),
                        ];

                        // Sort items so we see the most popular endpoints in the beginning
                        let mut sorted_endpoints =
                            ui_update.stats_endpoints.iter().collect::<Vec<_>>();
                        let mut sorted_addresses =
                            ui_update.stats_addresses.iter().collect::<Vec<_>>();
                        sorted_endpoints.sort_by(|a, b| b.1.cmp(a.1));
                        sorted_addresses.sort_by(|a, b| b.1.cmp(&a.1));

                        // Build table widgets with statistics over endpoints and IP addresses
                        for (k, v) in sorted_endpoints.iter() {
                            stats_endpoints.push(Row::StyledData(
                                vec![format!("/{}/*", k), v.to_string()].into_iter(),
                                value_style,
                            ));
                        }
                        for (k, v) in sorted_addresses.iter() {
                            stats_addresses.push(Row::StyledData(
                                vec![k.to_string(), v.to_string()].into_iter(),
                                value_style,
                            ));
                        }
                        // Build List widget with log samples
                        for log in ui_update.log_samples.into_iter() {
                            log_samples.push(Spans::from(Span::raw(log)));
                        }

                        // Build dynamic blocks of HTTP-codes statistics
                        let mut http_codes_to_stats: HashMap<u16, Rows> = HashMap::new();

                        for (http_code, stats) in ui_update.stats_http_codes.iter() {
                            let mut sorted_endpoints = stats.iter().collect::<Vec<_>>();
                            sorted_endpoints.sort_by(|a, b| b.1.cmp(a.1));
                            http_codes_to_stats.insert(
                                *http_code,
                                sorted_endpoints
                                    .iter()
                                    .map(|(k, v)| {
                                        Row::StyledData(
                                            vec![format!("/{}", k), v.to_string()].into_iter(),
                                            value_style,
                                        )
                                    })
                                    .collect(),
                            );
                        }
                        stats_http_codes = http_codes_to_stats.into_iter().collect::<Vec<_>>();
                        stats_http_codes.sort_by(|a, b| a.0.cmp(&b.0))
                    }
                    RenderMessage::Exit => exit_signal = true,
                }
            };

            let create_block = |title| {
                Block::default().borders(Borders::ALL).title(Span::styled(
                    title,
                    Style::default().add_modifier(Modifier::BOLD),
                ))
            };

            let alert = Paragraph::new(alert_text)
                .block(create_block("Alert"))
                .alignment(Alignment::Left)
                .style(alert_style)
                .wrap(Wrap { trim: true });

            let stats_general = Paragraph::new(stats_text)
                .block(create_block("General Stats"))
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

            // Dynamically create tables with HTTP-codes statistics
            let stats_http_codes: DynTables = stats_http_codes
                .into_iter()
                .map(|(code, stats)| {
                    Table::new(
                        vec![String::from("Endpoint"), String::from("Hits")].into_iter(),
                        (stats).into_iter(),
                    )
                    .block(Block::default().borders(Borders::ALL).title(Span::styled(
                        code.to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    )))
                    .widths(&[Constraint::Percentage(80), Constraint::Percentage(20)])
                    .column_spacing(1)
                })
                .collect();

            let log_samples = Paragraph::new(log_samples)
                .block(create_block("Log samples"))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });
            /* Divide space of the screen to different chunks */
            // Divide space horizontally
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(10), Constraint::Percentage(80)].as_ref())
                .split(f.size());

            // Divide top space vertically
            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                .split(chunks[0]);

            // Divide bottom space vertically
            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                .split(chunks[1]);
            let left_bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(bottom_chunks[0]);
            let right_bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                .split(bottom_chunks[1]);

            // Dynamically create screen chunks for HTTP-codes statistics
            let right_middle_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    vec![
                        Constraint::Percentage(
                            // Avoid division by zero at the first render
                            100 / if !stats_http_codes.is_empty() {
                                stats_http_codes.len()
                            } else {
                                1
                            } as u16
                        );
                        stats_http_codes.len()
                    ]
                    .as_ref(),
                )
                .split(right_bottom_chunks[0]);
            // Render everything!
            f.render_widget(alert, top_chunks[0]);
            f.render_widget(stats_general, top_chunks[1]);
            f.render_widget(stats_endpoints, left_bottom_chunks[0]);
            f.render_widget(stats_addresses, left_bottom_chunks[1]);
            f.render_widget(log_samples, right_bottom_chunks[1]);
            for (index, table) in stats_http_codes.into_iter().enumerate() {
                f.render_widget(table, right_middle_chunks[index]);
            }
        })?;
        if exit_signal {
            return Ok(());
        }
    }
}
