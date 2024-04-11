use std::future::Future;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use daemon_slayer_client::{ServiceManager, State};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub trait MenuHandler: Send + Sync {
    fn refresh_state(&mut self) -> impl Future<Output = ()> + Send;
    fn build_menu(&mut self) -> Menu;
    fn build_tray(&mut self, menu: &Menu) -> TrayIcon;
    fn update_menu(&self, menu: &Menu);
    fn handle_menu_event(&mut self, event: MenuEvent) -> impl Future<Output = ControlFlow> + Send;
    fn handle_tray_event(
        &mut self,
        event: TrayIconEvent,
    ) -> impl Future<Output = ControlFlow> + Send;
}

pub struct DefaultMenuHandler {
    manager: ServiceManager,
    icon_path: std::path::PathBuf,
    current_state: State,
    start_stop_id: MenuId,
    restart_id: MenuId,
    quit_id: MenuId,
}

impl MenuHandler for DefaultMenuHandler {
    async fn refresh_state(&mut self) {
        self.current_state = self.manager.status().await.unwrap().state;
    }

    fn build_menu(&mut self) -> Menu {
        let menu = Menu::new();
        let start_stop_text = get_start_stop_text(&self.current_state);
        let start_stop = MenuItem::new(start_stop_text, true, None);
        self.start_stop_id = start_stop.id().clone();
        let restart = MenuItem::new("Restart", true, None);
        self.restart_id = restart.id().clone();
        let quit = MenuItem::new("Quit", true, None);
        self.quit_id = quit.id().clone();
        menu.append_items(&[&start_stop, &restart, &quit]).unwrap();
        menu
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

    async fn handle_menu_event(&mut self, event: MenuEvent) -> ControlFlow {
        if event.id == self.start_stop_id {
            if self.current_state == State::Started {
                self.manager.stop().await.unwrap();
            } else {
                self.manager.start().await.unwrap();
            }
        } else if event.id == self.restart_id {
            self.manager.restart().await.unwrap();
        } else if event.id == self.quit_id {
            return ControlFlow::Exit;
        }

        ControlFlow::Poll
    }

    async fn handle_tray_event(&mut self, _event: TrayIconEvent) -> ControlFlow {
        ControlFlow::Poll
    }
}

pub struct Tray<T: MenuHandler + 'static> {
    menu_handler: T,
}

impl Tray<DefaultMenuHandler> {
    pub fn with_default_handler(manager: ServiceManager, icon_path: impl Into<PathBuf>) -> Self {
        Self {
            menu_handler: DefaultMenuHandler {
                manager,
                icon_path: icon_path.into(),
                current_state: State::NotInstalled,
                start_stop_id: MenuId::default(),
                restart_id: MenuId::default(),
                quit_id: MenuId::default(),
            },
        }
    }
}

impl<T: MenuHandler> Tray<T> {
    pub fn start(mut self) {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(self.menu_handler.refresh_state());

        let event_loop = EventLoopBuilder::new().build();

        let menu = self.menu_handler.build_menu();
        let mut tray_icon = Some(self.menu_handler.build_tray(&menu));

        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayIconEvent::receiver();

        let mut last_update_time = Instant::now();
        event_loop.run(move |_event, _, control_flow| {
            if let Ok(event) = menu_channel.try_recv() {
                *control_flow = handle.block_on(self.menu_handler.handle_menu_event(event));
                handle.block_on(self.menu_handler.refresh_state());
                self.menu_handler.update_menu(&menu)
            } else if let Ok(event) = tray_channel.try_recv() {
                *control_flow = handle.block_on(self.menu_handler.handle_tray_event(event));
                handle.block_on(self.menu_handler.refresh_state());
                self.menu_handler.update_menu(&menu)
            } else {
                let now = Instant::now();
                if now.duration_since(last_update_time) >= Duration::from_secs(1) {
                    handle.block_on(self.menu_handler.refresh_state());
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
