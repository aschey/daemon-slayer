use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use daemon_slayer_client::{Info, Manager, ServiceManager, State};
use daemon_slayer_core::health_check::HealthCheck;
use futures::{select, FutureExt, Stream, StreamExt};
use std::{
    error::Error,
    io::{self, Stdout},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tokio::io::{split, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing_ipc_widget::LogView;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

pub struct Console<'a> {
    manager: ServiceManager,
    info: Info,
    logs: LogView<'a>,
    button_index: usize,
    is_healthy: Option<bool>,
    health_check: Option<Box<dyn HealthCheck + Send + 'static>>,
    has_health_check: bool,
}

impl<'a> Console<'a> {
    pub fn new(manager: ServiceManager) -> Self {
        let info = manager.info().unwrap();
        let name = manager.name().to_owned();
        Self {
            manager,
            info,
            logs: LogView::new(name),
            button_index: 0,
            is_healthy: None,
            health_check: None,
            has_health_check: false,
        }
    }

    pub fn add_health_check(&mut self, health_check: Box<dyn HealthCheck + Send + 'static>) {
        self.health_check = Some(health_check);
        self.has_health_check = true;
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // create app and run it
        let res = self.run_app(&mut terminal).await;

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

    fn health_checker(
        mut health_checker: Box<dyn HealthCheck + Send + 'static>,
        tx: tokio::sync::mpsc::Sender<bool>,
    ) {
        tokio::spawn(async move {
            let mut is_healthy: Option<bool> = None;
            loop {
                match health_checker.invoke().await {
                    Ok(()) => {
                        if is_healthy != Some(true) {
                            is_healthy = Some(true);
                            let _ = tx.send(true).await;
                        }
                    }
                    Err(e) => {
                        if is_healthy != Some(false) {
                            is_healthy = Some(false);
                            let _ = tx.send(false).await;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    async fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (health_tx, mut health_rx) = tokio::sync::mpsc::channel(32);
        let name = self.manager.name().to_owned();

        if let Some(health_check) = self.health_check.take() {
            Self::health_checker(health_check, health_tx);
        }

        let mut event_reader = EventStream::new().fuse();
        let mut last_update = Instant::now();
        loop {
            if Instant::now().duration_since(last_update) > Duration::from_secs(1) {
                self.info = self.manager.info().unwrap();
                last_update = Instant::now();
            }

            if self.info.state == State::NotInstalled {
                self.button_index = 0;
            }
            terminal.draw(|f| self.ui(f))?;

            tokio::select! {
                _ = self.logs.update() => {}
                is_healthy = health_rx.recv() => {
                    self.is_healthy = is_healthy;
                }
                maybe_event = event_reader.next() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            if let Event::Key(key) = event {
                                match (key.modifiers, key.code) {
                                    (_, KeyCode::Char('q') | KeyCode::Esc) |
                                        (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Ok(()),
                                    (_, KeyCode::Down) =>  self.logs.next(),
                                    (_, KeyCode::Up) =>  self.logs.previous(),
                                    (_, KeyCode::Left) => {
                                        if self.button_index > 0 {
                                            self.button_index -= 1;
                                        }
                                    }
                                    (_, KeyCode::Right) => {
                                        if self.button_index < 4 {
                                            self.button_index += 1;
                                        }
                                    }
                                    (_, KeyCode::Enter) => {
                                        match self.button_index {
                                            0 => {
                                                if self.info.state == State::NotInstalled {
                                                    self.manager.install()?
                                                } else {
                                                    self.manager.uninstall()?;
                                                }
                                            },
                                            1 => {
                                                    self.manager.set_autostart_enabled(!self.info.autostart.unwrap_or(false))?;
                                                }
                                            2 => {
                                                if self.info.state == State::Stopped {
                                                    self.manager.start()?
                                                } else {
                                                    self.manager.stop()?;
                                                }
                                            },
                                            3 => {
                                                self.manager.restart()?;
                                            },
                                            _ => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        None => {}
                        _ => return Ok(())
                    }
                },
                _ =  tokio::time::sleep(Duration::from_millis(1000)) => {},
            };
        }
    }

    fn ui(&mut self, f: &mut Frame<CrosstermBackend<Stdout>>) {
        let size = f.size();

        // Main border
        let main_block = get_main_block();
        f.render_widget(main_block, size);

        let num_labels = if self.has_health_check { 5 } else { 4 };

        let (top_left, top_right, bottom) = get_main_sections(size, num_labels);

        let status_area = horizontal()
            .constraints([Constraint::Length(28), Constraint::Min(1)])
            .split(top_left);

        let status_block = bordered_block().border_style(reset_all()).title("Status");

        let label_area = get_label_area(num_labels, top_left);

        let state_label = get_label("State:");
        let state_value = match self.info.state {
            State::Started => get_label_value("Started", Color::Green),
            State::Stopped => get_label_value("Stopped", Color::Red),
            State::NotInstalled => get_label_value("Not Installed", Color::Blue),
        };

        let autostart_label = get_label("Autostart:");
        let autostart_value = match self.info.autostart {
            Some(true) => get_label_value("Enabled", Color::Blue),
            Some(false) => get_label_value("Disabled", Color::Yellow),
            None => get_label_value("N/A", Color::Reset),
        };
        let health_check_label = get_label("Health:");
        let health_check_value = match (self.is_healthy, &self.info.state) {
            (Some(true), State::Started) => get_label_value("Healthy", Color::Green),
            (Some(false), State::Started) => get_label_value("Unhealthy", Color::Red),
            _ => get_label_value("N/A", Color::Reset),
        };

        let pid_label = get_label("PID:");
        let pid = match self.info.pid {
            Some(pid) => pid.to_string(),
            None => "N/A".to_owned(),
        };
        let pid_value = get_label_value(&pid, Color::Reset);

        let exit_code_label = get_label("Exit Code:");
        let exit_code_value = match self.info.last_exit_code {
            Some(0) => get_label_value("0", Color::Green),
            Some(code) => get_label_value(code.to_string(), Color::Yellow),
            None => get_label_value("N/A", Color::Reset),
        };

        let mut index = 0;
        f.render_widget(state_label, label_area.0[index]);
        f.render_widget(state_value, label_area.1[index]);
        index += 1;

        f.render_widget(autostart_label, label_area.0[index]);
        f.render_widget(autostart_value, label_area.1[index]);
        index += 1;

        if self.has_health_check {
            f.render_widget(health_check_label, label_area.0[index]);
            f.render_widget(health_check_value, label_area.1[index]);
            index += 1;
        }

        f.render_widget(exit_code_label, label_area.0[index]);
        f.render_widget(exit_code_value, label_area.1[index]);
        index += 1;

        f.render_widget(pid_label, label_area.0[index]);
        f.render_widget(pid_value, label_area.1[index]);

        //  f.render_widget(logging_paragraph, info_section);
        f.render_widget(status_block, status_area[0]);

        let right_sections = vertical()
            .constraints([Constraint::Length(5), Constraint::Min(1)])
            .split(top_right);

        let button = Paragraph::new(vec![
            Spans::from(""),
            Spans::from(vec![
                Span::raw(" "),
                get_button(
                    if self.info.state == State::NotInstalled {
                        " install "
                    } else {
                        "uninstall"
                    },
                    Color::Blue,
                    self.button_index == 0,
                    false,
                ),
                Span::raw(" "),
                get_button(
                    if self.info.autostart.unwrap_or(false) {
                        "disable"
                    } else {
                        "enable "
                    },
                    Color::Blue,
                    self.button_index == 1,
                    self.info.state == State::NotInstalled,
                ),
                Span::raw(" "),
                get_button(
                    if self.info.state == State::Started {
                        "stop "
                    } else {
                        "start"
                    },
                    if self.info.state == State::Started {
                        Color::Red
                    } else {
                        Color::Green
                    },
                    self.button_index == 2,
                    self.info.state == State::NotInstalled,
                ),
                Span::raw(" "),
                get_button(
                    "restart",
                    Color::Magenta,
                    self.button_index == 3,
                    self.info.state == State::NotInstalled,
                ),
            ]),
        ])
        .block(bordered_block().title("Controls"));
        f.render_widget(button, right_sections[0]);

        self.logs.render(f, bottom);
    }
}

fn get_main_block() -> Block<'static> {
    bordered_block()
        .title("Daemon Slayer Console")
        .title_alignment(Alignment::Center)
}

fn get_button(text: &str, color: Color, selected: bool, disabled: bool) -> Span {
    let mut style = Style::default().bg(color).fg(Color::Rgb(240, 240, 240));

    if disabled {
        style = Style::default()
            .bg(Color::Rgb(75, 75, 75))
            .fg(Color::Rgb(175, 175, 175));
    } else if selected {
        style = style.add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK | Modifier::REVERSED);
    }
    Span::styled(format!(" {text} "), style)
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

fn get_main_sections(parent: Rect, num_labels: u16) -> (Rect, Rect, Rect) {
    let top_bottom = vertical()
        .constraints([Constraint::Length(num_labels + 4), Constraint::Min(1)])
        .vertical_margin(1)
        .horizontal_margin(2)
        .split(parent);
    let left_right = horizontal()
        .constraints([Constraint::Min(1), Constraint::Length(44)])
        .split(top_bottom[0]);
    (left_right[0], left_right[1], top_bottom[1])
}

fn get_label(label: &str) -> Paragraph {
    Paragraph::new(label)
        .alignment(Alignment::Right)
        .style(Style::default().add_modifier(Modifier::BOLD))
}

fn get_label_value(value: impl Into<String>, color: Color) -> Paragraph<'static> {
    Paragraph::new(value.into())
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
