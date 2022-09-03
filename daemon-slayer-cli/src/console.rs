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
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
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

    let (top_left, top_right, bottom) = get_main_sections(size);

    let num_labels = 4;
    let (label_section, info_section) = get_left_sections(top_left, num_labels);

    let status_area = horizontal()
        .constraints([Constraint::Length(25), Constraint::Min(1)])
        .split(label_section);

    let status_block = bordered_block().border_style(reset_all());

    let label_area = get_label_area(num_labels, top_left);

    let status_label = get_label("Status:");
    let status_value = get_label_value("Stopped", Color::Red);

    let autostart_label = get_label("Autostart:");
    let autostart_value = get_label_value("Enabled", Color::Blue);

    let health_check_label = get_label("Health:");
    let health_check_value = get_label_value("Healthy", Color::Green);

    let pid_label = get_label("PID:");
    let pid_value = get_label_value("12345", Color::Reset);

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
    f.render_widget(pid_label, label_area.0[3]);
    f.render_widget(pid_value, label_area.1[3]);
    f.render_widget(logging_paragraph, info_section);
    f.render_widget(status_block, status_area[0]);

    let right_sections = vertical()
        .constraints([Constraint::Length(5), Constraint::Min(1)])
        .split(top_right);

    let button = Paragraph::new(vec![
        Spans::from(""),
        Spans::from(vec![
            Span::raw(" "),
            get_button("start", Color::Green),
            Span::raw(" "),
            get_button("stop", Color::Red),
            Span::raw(" "),
            get_button("install", Color::Blue),
            Span::raw(" "),
            get_button("uninstall", Color::Blue),
            Span::raw(" "),
            get_button("run", Color::Magenta),
        ]),
    ])
    .block(bordered_block().title("Controls"));
    f.render_widget(button, right_sections[0]);

    let log_table =
        List::new(vec![ListItem::new("test log")]).block(bordered_block().title("Logs"));
    f.render_widget(log_table, bottom);
}

fn get_main_block() -> Block<'static> {
    bordered_block()
        .title("Daemon Slayer Console")
        .title_alignment(Alignment::Center)
}

fn get_button(text: &str, color: Color) -> Span {
    Span::styled(format!(" {text} "), Style::default().bg(color))
}

fn bordered_block() -> Block<'static> {
    Block::default()
        .borders(Borders::all())
        .border_type(BorderType::Rounded)
}

fn horizontal() -> Layout {
    Layout::default().direction(Direction::Horizontal)
}

fn vertical() -> Layout {
    Layout::default().direction(Direction::Vertical)
}

fn get_main_sections(parent: Rect) -> (Rect, Rect, Rect) {
    let top_bottom = vertical()
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .vertical_margin(1)
        .horizontal_margin(2)
        .split(parent);
    let left_right = horizontal()
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(top_bottom[0]);
    (left_right[0], left_right[1], top_bottom[1])
}

fn get_left_sections(left: Rect, num_labels: u16) -> (Rect, Rect) {
    let sections = vertical()
        .constraints([Constraint::Length(num_labels + 4), Constraint::Min(1)])
        .split(left);
    let info_section = horizontal()
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
    let bounds: Vec<_> = (0..num_labels).map(|_| Constraint::Length(1)).collect();
    let vert_slices = horizontal()
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
