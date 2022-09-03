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
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
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
    let main_block = get_main_block();
    f.render_widget(main_block, size);

    let (left, right) = get_left_right(size);

    let num_labels = 3;
    let (label_section, info_section) = get_left_sections(left, num_labels);

    let status_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(25), Constraint::Min(1)])
        .split(label_section);

    let status_block = bordered_block().border_style(reset_all());

    let label_area = get_label_area(num_labels, left);

    let status_label = get_label("Status:");
    let status_value = get_label_value("Stopped", Color::Red);

    let autostart_label = get_label("Autostart:");
    let autostart_value = get_label_value("Enabled", Color::Blue);

    let health_check_label = get_label("Health:");
    let health_check_value = get_label_value("Healthy", Color::Green);

    let text = vec![
        Spans::from(Span::raw("Event Viewer: Daemon Slayer Test Service")),
        Spans::from(Span::raw("Log file: C:\\\\Users\\bob\\test\\dir")),
    ];
    let logging_paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .style(reset_all())
                .title(Spans::from(Span::styled(
                    "Log Sources",
                    Style::default().add_modifier(Modifier::UNDERLINED),
                ))),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(status_label, label_area.0[0]);
    f.render_widget(status_value, label_area.1[0]);
    f.render_widget(autostart_label, label_area.0[1]);
    f.render_widget(autostart_value, label_area.1[1]);
    f.render_widget(health_check_label, label_area.0[2]);
    f.render_widget(health_check_value, label_area.1[2]);
    //f.render_widget(logging_label, log_section[0]);
    f.render_widget(logging_paragraph, info_section);
    f.render_widget(status_block, status_area[0]);

    let button = Paragraph::new(Spans::from(Span::styled(
        " start ",
        Style::default().bg(Color::Green),
    )))
    .block(bordered_block().title("Controls"));
    f.render_widget(button, right);
}

fn get_main_block() -> Block<'static> {
    bordered_block()
        .title("Daemon Slayer Console")
        .title_alignment(Alignment::Center)
}

fn bordered_block() -> Block<'static> {
    Block::default()
        .borders(Borders::all())
        .border_type(BorderType::Rounded)
}

fn get_left_right(parent: Rect) -> (Rect, Rect) {
    let sides = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .vertical_margin(1)
        .horizontal_margin(2)
        .split(parent);
    (sides[0], sides[1])
}

fn get_left_sections(left: Rect, num_labels: u16) -> (Rect, Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3 * num_labels), Constraint::Min(1)])
        .split(left);
    let info_section = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(sections[1]);
    (sections[0], info_section[1])
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

fn reset_all() -> Style {
    Style::default()
        .fg(Color::Reset)
        .bg(Color::Reset)
        .remove_modifier(Modifier::all())
}
