use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};

pub fn run() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f))?;

        if let Event::Key(key) = event::read()? {
            if let KeyCode::Char('q') = key.code {
                return Ok(());
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>) {
    let size = f.size();

    // Main border
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Daemon Slayer Console")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    f.render_widget(block, size);
    let sides = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(f.size());

    let vert_slices = Layout::default()
        .direction(Direction::Horizontal)
        .margin(4)
        .constraints([Constraint::Length(10), Constraint::Length(10)].as_ref())
        .split(sides[0]);

    let labels = Layout::default()
        .constraints([Constraint::Length(2), Constraint::Length(2)].as_ref())
        .split(vert_slices[0]);

    let values = Layout::default()
        .constraints([Constraint::Length(2), Constraint::Length(2)].as_ref())
        .horizontal_margin(1)
        .split(vert_slices[1]);

    let status_label = Paragraph::new("Status:")
        .alignment(Alignment::Right)
        .style(Style::default().add_modifier(Modifier::BOLD));

    let status_value = Paragraph::new("Stopped")
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::Red));

    let autostart_label = Paragraph::new("Autostart:")
        .alignment(Alignment::Right)
        .style(Style::default().add_modifier(Modifier::BOLD));

    let autostart_value = Paragraph::new("Enabled")
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::Blue));

    f.render_widget(status_label, labels[0]);
    f.render_widget(status_value, values[0]);
    f.render_widget(autostart_label, labels[1]);
    f.render_widget(autostart_value, values[1]);
}
