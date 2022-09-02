use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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
    let main_block = Block::default()
        .borders(Borders::ALL)
        .title("Daemon Slayer Console")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded);
    f.render_widget(main_block, size);

    let sides = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .vertical_margin(1)
        .horizontal_margin(2)
        .split(f.size());
    let left = sides[0];

    let num_labels = 3;
    let left_side = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3 * num_labels), Constraint::Min(1)])
        .split(left);
    let status_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(25), Constraint::Min(1)])
        .split(left_side[0]);

    let label_area = get_label_area(num_labels, left);

    let status_label = get_label("Status:");
    let status_value = get_label_value("Stopped", Color::Red);

    let autostart_label = get_label("Autostart:");
    let autostart_value = get_label_value("Enabled", Color::Blue);

    let health_check_label = get_label("Health:");
    let health_check_value = get_label_value("Healthy", Color::Green);

    f.render_widget(status_label, label_area.0[0]);
    f.render_widget(status_value, label_area.1[0]);
    f.render_widget(autostart_label, label_area.0[1]);
    f.render_widget(autostart_value, label_area.1[1]);
    f.render_widget(health_check_label, label_area.0[2]);
    f.render_widget(health_check_value, label_area.1[2]);

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Reset))
        .border_type(BorderType::Rounded);
    f.render_widget(status_block, status_area[0]);
}

fn get_label(label: &str) -> Paragraph {
    Paragraph::new(label)
        .alignment(Alignment::Right)
        .style(Style::default().add_modifier(Modifier::BOLD))
}

fn get_label_value(value: &str, color: Color) -> Paragraph {
    Paragraph::new(value)
        .alignment(Alignment::Left)
        .style(Style::default().fg(color))
}

fn get_label_area(num_labels: u16, left: Rect) -> (Vec<Rect>, Vec<Rect>) {
    let bounds: Vec<_> = (0..num_labels).map(|_| Constraint::Length(2)).collect();
    let vert_slices = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Length(10), Constraint::Length(10)].as_ref())
        .split(left);

    let labels = Layout::default()
        .constraints(&*bounds)
        .split(vert_slices[0]);

    let values = Layout::default()
        .constraints(&*bounds)
        .horizontal_margin(1)
        .split(vert_slices[1]);

    (labels, values)
}
