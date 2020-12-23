use std::collections::HashMap;
use std::io;
use std::sync::mpsc::Receiver;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use tui::backend::Backend;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders};
use tui::Terminal;

pub enum RenderMessage {
    UI(UIUpdate),
    Exit(bool),
}

pub struct UIUpdate {
    pub alert: Alert,
    pub stats: HashMap<String, i64>,
}

pub struct Alert {
    pub active: bool,
    pub value: i64,
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

pub fn draw<B>(terminal: &mut Terminal<B>, rx: Receiver<RenderMessage>) -> Result<(), io::Error>
where
    B: Backend,
{
    //    let events = Events::new();
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(10),
                        Constraint::Percentage(80),
                        Constraint::Percentage(10),
                    ]
                    .as_ref(),
                )
                .split(f.size());
            let block = Block::default()
                .title("Current values")
                .borders(Borders::ALL);
            f.render_widget(block, chunks[0]);
            let block = Block::default().title("Block 2").borders(Borders::ALL);
            f.render_widget(block, chunks[1]);
        })?;
    }
}
