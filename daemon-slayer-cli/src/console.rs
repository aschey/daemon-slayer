use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use daemon_slayer_client::{Manager, ServiceManager, Status};
use futures::{select, FutureExt, Stream, StreamExt};
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use std::{
    error::Error,
    io::{self, Stdout},
    pin::Pin,
    time::{Duration, Instant},
};
use tokio::io::{split, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
trait AsyncStream: AsyncRead + AsyncWrite {}
pub struct Console<'a> {
    manager: ServiceManager,
    last_update: Instant,
    status: Status,
    logs: Vec<ListItem<'a>>,
}

impl<'a> Console<'a> {
    pub fn new(manager: ServiceManager) -> Self {
        println!("NEW");

        let status = manager.query_status().unwrap();
        Self {
            manager,
            status,
            logs: vec![],
            last_update: Instant::now(),
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
    ) -> io::Result<()> {
        let mut reader = EventStream::new();

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            let mut endpoint = Endpoint::new("/tmp/daemon_slayer.sock".to_owned());
            endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

            let incoming = endpoint.incoming().expect("failed to open new socket");
            futures::pin_mut!(incoming);

            while let Some(result) = incoming.next().await {
                match result {
                    Ok(stream) => {
                        let (mut reader, mut writer) = split(stream);
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            loop {
                                let mut buf = [0u8; 256];

                                let bytes = match reader.read(&mut buf).await {
                                    Ok(0) => break,
                                    Ok(bytes) => bytes,
                                    Err(_) => break,
                                };

                                let text = String::from_utf8(buf[0..bytes].to_vec())
                                    .unwrap()
                                    .replace('\n', "")
                                    .into_text()
                                    .unwrap();

                                tx.send(ListItem::new(text)).await;
                            }
                        });
                    }
                    _ => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        });
        let mut log_stream_running = true;
        loop {
            terminal.draw(|f| self.ui(f))?;

            //tokio::time::sleep(Duration::from_millis(10)).await;
            tokio::select! {
                log = rx.recv(), if log_stream_running => {

                    if let Some(log) = log {
                        let mut new_logs = vec![log];
                        new_logs.extend_from_slice(&self.logs);
                        self.logs = new_logs;
                    } else {
                        log_stream_running = false;
                    }
                }
                maybe_event = reader.next() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            if let Event::Key(key) = event {
                                if let KeyCode::Char('q') = key.code {
                                    return Ok(());
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
            self.status = self.manager.query_status().unwrap();
            self.last_update = Instant::now();
        }
        // Main border
        let main_block = get_main_block();
        f.render_widget(main_block, size);

        let (top_left, top_right, bottom) = get_main_sections(size);

        let num_labels = 4;

        let status_area = horizontal()
            .constraints([Constraint::Length(28), Constraint::Min(1)])
            .split(top_left);

        let status_block = bordered_block().border_style(reset_all());

        let label_area = get_label_area(num_labels, top_left);

        let status_label = get_label("Status:");
        let status_value = match self.status {
            Status::Started => get_label_value("Started", Color::Green),
            Status::Stopped => get_label_value("Stopped", Color::Red),
            Status::NotInstalled => get_label_value("Not Installed", Color::Blue),
        };

        let autostart_label = get_label("Autostart:");
        let autostart_value = get_label_value("Enabled", Color::Blue);

        let health_check_label = get_label("Health:");
        let health_check_value = get_label_value("Healthy", Color::Green);

        let pid_label = get_label("PID:");
        let pid_value = get_label_value("12345", Color::Reset);

        f.render_widget(status_label, label_area.0[0]);
        f.render_widget(status_value, label_area.1[0]);
        f.render_widget(autostart_label, label_area.0[1]);
        f.render_widget(autostart_value, label_area.1[1]);
        f.render_widget(health_check_label, label_area.0[2]);
        f.render_widget(health_check_value, label_area.1[2]);
        f.render_widget(pid_label, label_area.0[3]);
        f.render_widget(pid_value, label_area.1[3]);
        //  f.render_widget(logging_paragraph, info_section);
        f.render_widget(status_block, status_area[0]);

        let right_sections = vertical()
            .constraints([Constraint::Length(5), Constraint::Min(1)])
            .split(top_right);

        let button = Paragraph::new(vec![
            Spans::from(""),
            Spans::from(vec![
                Span::raw(" "),
                get_button("start", Color::Green, true),
                Span::raw(" "),
                get_button("stop", Color::Red, false),
                Span::raw(" "),
                get_button("install", Color::Blue, false),
                Span::raw(" "),
                get_button("uninstall", Color::Blue, false),
                Span::raw(" "),
                get_button("run", Color::Magenta, false),
            ]),
        ])
        .block(bordered_block().title("Controls"));
        f.render_widget(button, right_sections[0]);

        let log_table = List::new(&*self.logs).block(bordered_block().title("Logs"));
        f.render_widget(log_table, bottom);
    }
}

fn get_main_block() -> Block<'static> {
    bordered_block()
        .title("Daemon Slayer Console")
        .title_alignment(Alignment::Center)
}

fn get_button(text: &str, color: Color, selected: bool) -> Span {
    let mut style = Style::default().bg(color);
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
        .constraints([Constraint::Length(8), Constraint::Min(1)])
        .vertical_margin(1)
        .horizontal_margin(2)
        .split(parent);
    let left_right = horizontal()
        .constraints([Constraint::Min(1), Constraint::Length(47)])
        .split(top_bottom[0]);
    (left_right[0], left_right[1], top_bottom[1])
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
