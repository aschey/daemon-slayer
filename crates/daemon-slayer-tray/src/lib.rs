use async_trait::async_trait;
use daemon_slayer_client::{ServiceManager, State};
use std::path::PathBuf;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
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
    async fn handle_menu_event(&mut self, event: MenuEvent) -> ControlFlow;
    async fn handle_tray_event(&mut self, event: TrayEvent) -> ControlFlow;
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

    async fn handle_menu_event(&mut self, event: MenuEvent) -> ControlFlow {
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
                return ControlFlow::Exit;
            }
            _ => {}
        }

        ControlFlow::Poll
    }

    async fn handle_tray_event(&mut self, _event: TrayEvent) -> ControlFlow {
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
                start_stop_id: 0,
                restart_id: 0,
                quit_id: 0,
            },
        }
    }
}

impl<T: MenuHandler> Tray<T> {
    pub fn start(mut self) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(self.menu_handler.refresh_state());

        let menu = self.menu_handler.build_menu();
        let mut tray_icon = Some(self.menu_handler.build_tray(&menu));

        let event_loop = EventLoopBuilder::new().build();

        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayEvent::receiver();

        event_loop.run(move |_event, _, control_flow| {
            if let Ok(event) = menu_channel.try_recv() {
                rt.block_on(self.menu_handler.refresh_state());
                *control_flow = rt.block_on(self.menu_handler.handle_menu_event(event));
                self.menu_handler.update_menu(&menu)
            } else if let Ok(event) = tray_channel.try_recv() {
                rt.block_on(self.menu_handler.refresh_state());
                *control_flow = rt.block_on(self.menu_handler.handle_tray_event(event));
                self.menu_handler.update_menu(&menu)
            } else {
                *control_flow = ControlFlow::Poll;
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
