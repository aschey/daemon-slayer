use std::io::{self, Stdout};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind,
    KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use daemon_slayer_client::{ServiceManager, State, Status};
use daemon_slayer_core::config::{Accessor, CachedConfig};
use daemon_slayer_core::health_check::HealthCheck;
use daemon_slayer_core::server::background_service::{
    self, BackgroundService, Manager, ServiceContext,
};
use daemon_slayer_core::{BoxedError, CancellationToken};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::{Frame, Terminal};
pub use tilia::transport::docker::LogSource as DockerLogSource;
use tilia::transport_async::ipc::ServerId;
use tilia_widget::LogView;
use tilia_widget::transport::docker::docker_client;
use tilia_widget::transport::ipc_client;
use tokio::sync::mpsc;
use tokio_util::future::FutureExt;

#[derive(daemon_slayer_core::Mergeable, Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "config", derive(confique::Config, serde::Deserialize))]
pub struct UserConfig {
    #[cfg_attr(feature = "config", config(default = true))]
    pub enable_health_check: bool,
    #[cfg_attr(feature = "config", config(default = 1))]
    pub health_check_interval_seconds: u64,
}

struct HealthChecker {
    user_config: CachedConfig<UserConfig>,
    health_check: Box<dyn HealthCheck + Send + 'static>,
    tx: mpsc::Sender<bool>,
}

impl HealthChecker {
    fn new(
        user_config: CachedConfig<UserConfig>,
        health_check: Box<dyn HealthCheck + Send + 'static>,
        tx: mpsc::Sender<bool>,
    ) -> Self {
        Self {
            user_config,
            health_check,
            tx,
        }
    }
}

impl BackgroundService for HealthChecker {
    fn name(&self) -> &str {
        "health_check_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut is_healthy: Option<bool> = None;
        while let Some(res) = self
            .health_check
            .invoke()
            .with_cancellation_token(context.cancellation_token())
            .await
        {
            match res {
                Ok(()) => {
                    if is_healthy != Some(true) {
                        is_healthy = Some(true);
                        let _ = self.tx.send(true).await;
                    }
                }
                Err(_e) => {
                    if is_healthy != Some(false) {
                        is_healthy = Some(false);
                        let _ = self.tx.send(false).await;
                    }
                }
            }
            let sleep_time =
                Duration::from_secs(self.user_config.load().health_check_interval_seconds);
            tokio::time::sleep(sleep_time).await;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum LogSource {
    Ipc,
    Container { output_source: DockerLogSource },
}

pub struct Console {
    manager: ServiceManager,
    info: Status,
    logs: LogView<'static>,
    button_index: usize,
    is_healthy: Option<bool>,
    health_check: Option<Box<dyn HealthCheck + Send + Sync + 'static>>,
    has_health_check: bool,
    user_config: CachedConfig<UserConfig>,
    event_fn: Option<Box<dyn FnOnce(ServiceContext) + Send>>,
}

impl Console {
    pub async fn new(manager: ServiceManager, log_source: LogSource) -> Self {
        let info = manager.status().await.unwrap();
        let name = manager.label().application.to_owned();
        Self {
            manager,
            info,
            logs: match log_source {
                LogSource::Ipc => LogView::new(ipc_client(ServerId::new(name + "_logger"))),
                LogSource::Container { output_source } => {
                    LogView::new(docker_client(name, output_source))
                }
            },
            button_index: 0,
            is_healthy: None,
            health_check: None,
            has_health_check: false,
            user_config: Default::default(),
            event_fn: None,
        }
    }

    pub fn with_config<A>(mut self, service: A) -> Self
    where
        A: Accessor<UserConfig> + Clone + Unpin + 'static,
    {
        self.user_config = service.access();
        self
    }

    pub fn with_configure_services<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(ServiceContext) + Send + 'static,
    {
        self.event_fn = Some(Box::new(configure));
        self
    }

    pub fn with_health_check(
        mut self,
        health_check: Box<dyn HealthCheck + Send + Sync + 'static>,
    ) -> Self {
        self.health_check = Some(health_check);
        self.has_health_check = true;
        self
    }

    pub async fn run(self) -> Result<(), BoxedError> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // create app and run it
        let res = self.run_app(&mut terminal, CancellationToken::new()).await;

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{err:?}")
        }

        Ok(())
    }

    async fn run_app(
        mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        cancellation_token: CancellationToken,
    ) -> Result<(), BoxedError> {
        let manager = Manager::new(
            cancellation_token.child_token(),
            background_service::Settings::default(),
        );
        let context = manager.get_context();
        if let Some(event_fn) = self.event_fn.take() {
            event_fn(context);
        }

        let (health_tx, mut health_rx) = mpsc::channel(32);

        if let Some(health_check) = self.health_check.take() {
            let health_checker =
                HealthChecker::new(self.user_config.clone(), health_check, health_tx);
            let context = manager.get_context();
            context.spawn(health_checker);
        }

        let mut event_reader = EventStream::new().fuse();
        let mut last_update = Instant::now();
        loop {
            if Instant::now().duration_since(last_update) > Duration::from_secs(1) {
                self.info = self.manager.status().await.unwrap();
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
                    if let Some(event) = maybe_event {
                        match event {
                            Ok(Event::Key(key)) => {
                                if key.kind == KeyEventKind::Press {
                                    match (key.modifiers, key.code) {
                                        (_, KeyCode::Char('q') | KeyCode::Esc) |
                                            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                                                cancellation_token.cancel();
                                                return Ok(());
                                            },
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
                                                        self.manager.install().await?
                                                    } else {
                                                        self.manager.uninstall().await?;
                                                    }
                                                },
                                                1 => {
                                                        if self.info.autostart.unwrap_or(false) {
                                                            self.manager.disable_autostart().await?;
                                                        } else {
                                                            self.manager.enable_autostart().await?;
                                                        }
                                                    }
                                                2 => {
                                                    if self.info.state == State::Stopped {
                                                        self.manager.start().await?
                                                    } else {
                                                        self.manager.stop().await?;
                                                    }
                                                },
                                                3 => {
                                                    self.manager.restart().await?;
                                                },
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Err(_e) => return Ok(()),
                            _ => {}
                        }
                    }
                },
                _ =  tokio::time::sleep(Duration::from_millis(1000)) => {},
            };
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let size = f.area();

        // Main border
        let main_block = get_main_block();
        f.render_widget(main_block, size);

        let show_health_check = self.user_config.load().enable_health_check;
        let num_labels = if show_health_check { 5 } else { 4 };

        let (top_left, top_right, bottom) = get_main_sections(size, num_labels);

        let status_area = horizontal()
            .constraints([Constraint::Length(28), Constraint::Min(1)])
            .split(top_left);

        let status_block = bordered_block().border_style(reset_all()).title("Status");

        let label_area = get_label_area(num_labels, top_left);

        let state_label = get_label("State:");
        let state_value = match self.info.state {
            State::Started => get_label_value("Started", Color::Green),
            State::Listening => get_label_value("Listening", Color::Cyan),
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

        if show_health_check {
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
            Line::from(""),
            Line::from(vec![
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
                    if self.info.state == State::Started || self.info.state == State::Listening {
                        "stop "
                    } else {
                        "start"
                    },
                    if self.info.state == State::Started || self.info.state == State::Listening {
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

fn get_button(text: &str, color: Color, selected: bool, disabled: bool) -> Span<'_> {
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

fn get_label(label: &str) -> Paragraph<'_> {
    Paragraph::new(label)
        .alignment(Alignment::Right)
        .style(Style::default().add_modifier(Modifier::BOLD))
}

fn get_label_value(value: impl Into<String>, color: Color) -> Paragraph<'static> {
    Paragraph::new(value.into())
        .alignment(Alignment::Left)
        .style(Style::default().fg(color))
}

fn get_label_area(num_labels: u16, left: Rect) -> (Rc<[Rect]>, Rc<[Rect]>) {
    let bounds: Vec<_> = (0..num_labels).map(|_| Constraint::Length(1)).collect();
    let vert_slices = horizontal()
        .margin(2)
        .constraints([Constraint::Length(10), Constraint::Min(0)].as_ref())
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
