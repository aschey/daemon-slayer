use async_trait::async_trait;
use daemon_slayer_client::{ServiceManager, State};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};
use tray_icon::{
    icon::Icon,
    menu::{Menu, MenuEvent, MenuItem},
    TrayEvent, TrayIcon, TrayIconBuilder,
};

#[async_trait]
pub trait MenuHandler: Send + Sync {
    async fn refresh_state(&mut self);
    fn build_menu(&mut self) -> Menu;
    fn build_tray(&mut self, menu: &Menu) -> TrayIcon;
    fn update_menu(&self, menu: &Menu);
    async fn handle_event(&mut self, event: TrayIconEvent) -> Continue;
}

pub struct Continue(pub bool);

#[derive(Debug)]
pub enum TrayIconEvent {
    Menu(MenuEvent),
    Tray(TrayEvent),
}

pub struct DefaultMenuHandler {
    manager: ServiceManager,
    icon_path: std::path::PathBuf,
    current_state: State,
    start_stop_id: u32,
    restart_id: u32,
    quit_id: u32,
}

#[async_trait]
impl MenuHandler for DefaultMenuHandler {
    async fn refresh_state(&mut self) {
        self.current_state = self.manager.info().await.unwrap().state;
    }

    fn build_menu(&mut self) -> Menu {
        let menu = Menu::new();
        let start_stop_text = get_start_stop_text(&self.current_state);
        let start_stop = MenuItem::new(start_stop_text, true, None);
        self.start_stop_id = start_stop.id();
        let restart = MenuItem::new("Restart", true, None);
        self.restart_id = restart.id();
        let quit = MenuItem::new("Quit", true, None);
        self.quit_id = quit.id();
        menu.append_items(&[&start_stop, &restart, &quit]);
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
            .as_any()
            .downcast_ref::<MenuItem>()
            .unwrap()
            .set_text(get_start_stop_text(&self.current_state));
    }

    async fn handle_event(&mut self, event: TrayIconEvent) -> Continue {
        if let TrayIconEvent::Menu(event) = event {
            match event.id {
                id if id == self.start_stop_id => {
                    if self.current_state == State::Started {
                        self.manager.stop().await.unwrap();
                    } else {
                        self.manager.start().await.unwrap();
                    }
                }
                id if id == self.restart_id => {
                    self.manager.restart().await.unwrap();
                }
                id if id == self.quit_id => {
                    return Continue(false);
                }
                _ => {}
            }
        }
        Continue(true)
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
                start_stop_id: 0,
                restart_id: 0,
                quit_id: 0,
            },
        }
    }
}

impl<T: MenuHandler> Tray<T> {
    pub async fn start(mut self) {
        self.menu_handler.refresh_state().await;
        let menu_handler = Arc::new(RwLock::new(self.menu_handler));
        let menu_handler_ = menu_handler.clone();
        let (menu_tx, mut menu_rx) = tokio::sync::mpsc::channel(32);
        let (tray_tx, mut tray_rx) = tokio::sync::mpsc::channel(32);

        #[cfg(target_os = "linux")]
        let (tx, rx) = gtk::glib::MainContext::channel(gtk::glib::PRIORITY_DEFAULT);
        #[cfg(target_os = "linux")]
        std::thread::spawn(move || {
            gtk::init().unwrap();
            let menu = menu_handler.write().unwrap().build_menu();
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| menu_tx.try_send(e).unwrap()));
            TrayEvent::set_event_handler(Some(move |e| tray_tx.try_send(e).unwrap()));
            let _tray_icon = menu_handler.write().unwrap().build_tray(&menu);
            rx.attach(None, move |_| {
                menu_handler.read().unwrap().update_menu(&menu);
                gtk::prelude::Continue(true)
            });
            gtk::main();
        });

        #[cfg(not(target_os = "linux"))]
        let menu = menu_handler.write().unwrap().build_menu();
        #[cfg(not(target_os = "linux"))]
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| menu_tx.try_send(e).unwrap()));
        #[cfg(not(target_os = "linux"))]
        TrayEvent::set_event_handler(Some(move |e| tray_tx.try_send(e).unwrap()));
        #[cfg(not(target_os = "linux"))]
        let _tray_icon = menu_handler.write().unwrap().build_tray(&menu);

        #[cfg(target_os = "linux")]
        let handle_state_change = || tx.send(()).unwrap();
        #[cfg(not(target_os = "linux"))]
        let handle_state_change = || menu_handler.read().unwrap().update_menu(&menu);

        loop {
            tokio::select! {
                Some(event) = menu_rx.recv() => {
                    menu_handler_.write().unwrap().refresh_state().await;
                    if let Continue(false) =
                        menu_handler_.write().unwrap().handle_event(TrayIconEvent::Menu(event)).await {
                        return;
                    }
                    menu_handler_.write().unwrap().refresh_state().await;
                    handle_state_change();
                }
                Some(event) = tray_rx.recv() => {
                    menu_handler_.write().unwrap().refresh_state().await;
                    if let Continue(false) =
                        menu_handler_.write().unwrap().handle_event(TrayIconEvent::Tray(event)).await {
                        return;
                    }
                    menu_handler_.write().unwrap().refresh_state().await;
                    handle_state_change();
                }
                _ = tokio:: time:: sleep(Duration::from_secs(1)) => {
                    menu_handler_.write().unwrap().refresh_state().await;
                    handle_state_change();
                }
            }
        }
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
