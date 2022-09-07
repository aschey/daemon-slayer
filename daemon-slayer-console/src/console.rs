use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use daemon_slayer_client::{Info, Manager, ServiceManager, State};
use futures::{select, FutureExt, Stream, StreamExt};

use std::{
    error::Error,
    io::{self, Stdout},
    pin::Pin,
    time::{Duration, Instant},
};
use tokio::io::{split, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing_ipc::run_ipc_server;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::stateful_list::StatefulList;

pub struct Console<'a> {
    manager: ServiceManager,
    last_update: Instant,
    info: Info,
    logs: StatefulList<'a>,
    button_index: usize,
}

impl<'a> Console<'a> {
    pub fn new(manager: ServiceManager) -> Self {
        let info = manager.info().unwrap();
        Self {
            manager,
            info,
            logs: StatefulList::new(),
            last_update: Instant::now(),
            button_index: 0,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
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

    async fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn Error>> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let name = self.manager.name().to_owned();
        tokio::spawn(async move {
            run_ipc_server(&name, tx).await;
        });
        let mut log_stream_running = true;
        let mut event_reader = EventStream::new().fuse();
        loop {
            terminal.draw(|f| self.ui(f))?;

            tokio::select! {
                log = rx.recv(), if log_stream_running => {

                    if let Some(log) = log {
                        let text = ListItem::new(log.into_text().unwrap());
                        self.logs.add_item(text);
                    } else {
                        log_stream_running = false;
                    }
                }
                maybe_event = event_reader.next() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            if let Event::Key(key) = event {
                                match key.code {
                                    KeyCode::Char('q') => return Ok(()),
                                    KeyCode::Down =>  self.logs.next(),
                                    KeyCode::Up =>  self.logs.previous(),
                                    KeyCode::Left => {
                                        if self.button_index > 0 {
                                            self.button_index -= 1;
                                        }
                                    }
                                    KeyCode::Right => {
                                        if self.button_index < 4 {
                                            self.button_index += 1;
                                        }
                                    }
                                    KeyCode::Enter => {
                                        match self.button_index {
                                            0 => {
                                                if self.info.state == State::Stopped {
                                                    self.manager.start()?
                                                } else {
                                                    self.manager.stop()?;
                                                }
                                            },
                                            1 => {self.manager.restart()?;}
                                            2 => {
                                                if self.info.state == State::NotInstalled {
                                                    self.manager.install()?
                                                } else {
                                                    self.manager.uninstall()?;
                                                }
                                            },
                                            3 => {
                                                self.manager.set_autostart_enabled(!self.info.autostart.unwrap_or(false))?;
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
        if Instant::now().duration_since(self.last_update) > Duration::from_secs(1) {
            self.info = self.manager.info().unwrap();
            self.last_update = Instant::now();
        }
        // Main border
        let main_block = get_main_block();
        f.render_widget(main_block, size);

        let (top_left, top_right, bottom) = get_main_sections(size);

        let num_labels = 5;

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
            None => get_label_value("N/A", Color::Yellow),
        };
        let health_check_label = get_label("Health:");
        let health_check_value = get_label_value("Healthy", Color::Green);

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

        f.render_widget(state_label, label_area.0[0]);
        f.render_widget(state_value, label_area.1[0]);
        f.render_widget(autostart_label, label_area.0[1]);
        f.render_widget(autostart_value, label_area.1[1]);
        f.render_widget(health_check_label, label_area.0[2]);
        f.render_widget(health_check_value, label_area.1[2]);
        f.render_widget(exit_code_label, label_area.0[3]);
        f.render_widget(exit_code_value, label_area.1[3]);
        f.render_widget(pid_label, label_area.0[4]);
        f.render_widget(pid_value, label_area.1[4]);

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
                    if self.info.state == State::Stopped {
                        "start"
                    } else {
                        "stop "
                    },
                    if self.info.state == State::Stopped {
                        Color::Green
                    } else {
                        Color::Red
                    },
                    self.button_index == 0,
                ),
                Span::raw(" "),
                get_button("restart", Color::Magenta, self.button_index == 1),
                Span::raw(" "),
                get_button(
                    if self.info.state == State::NotInstalled {
                        " install "
                    } else {
                        "uninstall"
                    },
                    Color::Blue,
                    self.button_index == 2,
                ),
                Span::raw(" "),
                get_button(
                    if self.info.autostart.unwrap_or(false) {
                        "disable"
                    } else {
                        "enable "
                    },
                    Color::Blue,
                    self.button_index == 3,
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

fn get_button(text: &str, color: Color, selected: bool) -> Span {
    let mut style = Style::default().bg(color).fg(Color::Rgb(240, 240, 240));
    if selected {
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

fn get_main_sections(parent: Rect) -> (Rect, Rect, Rect) {
    let top_bottom = vertical()
        .constraints([Constraint::Length(9), Constraint::Min(1)])
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
