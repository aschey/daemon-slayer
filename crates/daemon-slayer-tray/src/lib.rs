use std::path::PathBuf;
use std::time::{Duration, Instant};

use daemon_slayer_client::{ServiceManager, State};
pub use tao::event_loop;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};
pub use tray_icon;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub trait MenuHandler {
    fn refresh_state(&mut self);
    fn get_menu(&mut self) -> Menu;
    fn build_tray(&mut self, menu: &Menu) -> TrayIcon;
    fn update_menu(&self, menu: &Menu);
    fn handle_menu_event(&mut self, event: MenuEvent) -> ControlFlow;
    fn handle_tray_event(&mut self, event: TrayIconEvent) -> ControlFlow;
}

pub struct DefaultMenuHandler {
    tx: mpsc::Sender<Command>,
    icon_path: std::path::PathBuf,
    current_state: State,
    menu: Menu,
    start_stop_id: MenuId,
    restart_id: MenuId,
    quit_id: MenuId,
}

impl DefaultMenuHandler {
    pub fn new(manager: ServiceManager, icon_path: PathBuf) -> Self {
        let menu = Menu::new();
        let start_stop_text = get_start_stop_text(&State::NotInstalled);
        let start_stop = MenuItem::new(start_stop_text, true, None);
        let restart = MenuItem::new("Restart", true, None);
        let quit = MenuItem::new("Quit", true, None);
        menu.append_items(&[&start_stop, &restart, &quit]).unwrap();

        let (tx, rx) = mpsc::channel(32);
        let _handle = Handle::current().enter();
        tokio::spawn(service_handler(manager, rx));
        Self {
            tx,
            icon_path,
            start_stop_id: start_stop.id().clone(),
            restart_id: restart.id().clone(),
            quit_id: quit.id().clone(),
            menu,
            current_state: State::NotInstalled,
        }
    }
}

enum Command {
    Start,
    Stop,
    Restart,
    State(oneshot::Sender<State>),
}

async fn service_handler(manager: ServiceManager, mut rx: mpsc::Receiver<Command>) {
    while let Some(command) = rx.recv().await {
        match command {
            Command::Start => {
                manager.start().await.unwrap();
            }
            Command::Stop => {
                manager.stop().await.unwrap();
            }
            Command::Restart => {
                manager.restart().await.unwrap();
            }
            Command::State(res) => {
                res.send(manager.status().await.unwrap().state).unwrap();
            }
        }
    }
}

impl MenuHandler for DefaultMenuHandler {
    fn refresh_state(&mut self) {
        let (state_tx, state_rx) = oneshot::channel();
        self.tx.blocking_send(Command::State(state_tx)).unwrap();
        self.current_state = state_rx.blocking_recv().unwrap();
    }

    fn get_menu(&mut self) -> Menu {
        self.menu.clone()
    }

    fn build_tray(&mut self, menu: &Menu) -> TrayIcon {
        TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_icon(load_icon(&self.icon_path))
            .build()
            .unwrap()
    }

    fn update_menu(&self, menu: &Menu) {
        menu.items()[0]
            .as_menuitem_unchecked()
            .set_text(get_start_stop_text(&self.current_state));
    }

    fn handle_menu_event(&mut self, event: MenuEvent) -> ControlFlow {
        if event.id == self.start_stop_id {
            if self.current_state == State::Started {
                self.tx.blocking_send(Command::Stop).unwrap();
            } else {
                self.tx.blocking_send(Command::Start).unwrap();
            }
        } else if event.id == self.restart_id {
            self.tx.blocking_send(Command::Restart).unwrap();
        } else if event.id == self.quit_id {
            return ControlFlow::Exit;
        }

        ControlFlow::Poll
    }

    fn handle_tray_event(&mut self, _event: TrayIconEvent) -> ControlFlow {
        ControlFlow::Poll
    }
}

pub struct Tray<T: MenuHandler + 'static> {
    menu_handler: T,
}

impl Tray<DefaultMenuHandler> {
    pub fn with_default_handler<P>(manager: ServiceManager, icon_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            menu_handler: DefaultMenuHandler::new(manager, icon_path.into()),
        }
    }
}

impl<T: MenuHandler> Tray<T> {
    pub fn with_handler(handler: T) -> Self {
        Self {
            menu_handler: handler,
        }
    }

    pub fn start(mut self) {
        self.menu_handler.refresh_state();

        let event_loop = EventLoopBuilder::new().build();

        let menu = self.menu_handler.get_menu();
        self.menu_handler.refresh_state();
        self.menu_handler.update_menu(&menu);
        let mut tray_icon = Some(self.menu_handler.build_tray(&menu));

        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayIconEvent::receiver();

        let mut last_update_time = Instant::now();
        event_loop.run(move |_event, _, control_flow| {
            if let Ok(event) = menu_channel.try_recv() {
                *control_flow = self.menu_handler.handle_menu_event(event);
                self.menu_handler.refresh_state();
                self.menu_handler.update_menu(&menu)
            } else if let Ok(event) = tray_channel.try_recv() {
                *control_flow = self.menu_handler.handle_tray_event(event);
                self.menu_handler.refresh_state();
                self.menu_handler.update_menu(&menu)
            } else {
                let now = Instant::now();
                if now.duration_since(last_update_time) >= Duration::from_secs(1) {
                    self.menu_handler.refresh_state();
                    last_update_time = now;
                }

                *control_flow = ControlFlow::WaitUntil(
                    Instant::now()
                        .checked_add(Duration::from_millis(10))
                        .unwrap(),
                );
            }

            if *control_flow == ControlFlow::Exit {
                tray_icon.take();
            }
        });
    }
}

pub fn get_start_stop_text(state: &State) -> &str {
    if state == &State::Started {
        "Stop"
    } else {
        "Start"
    }
}

pub fn load_icon(path: &std::path::Path) -> Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path).unwrap().into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap()
}
