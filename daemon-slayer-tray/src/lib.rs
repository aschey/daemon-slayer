use std::time::{Duration, Instant};

use daemon_slayer_client::{Manager, ServiceManager, State};
#[cfg(target_os = "macos")]
use tao::platform::macos::{SystemTrayBuilderExtMacOS, SystemTrayExtMacOS};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu as Menu, MenuItemAttributes, MenuType},
    system_tray::SystemTrayBuilder,
    TrayId,
};

pub fn start(icon_path: &std::path::Path, manager: ServiceManager) {
    let event_loop = EventLoop::new();
    let current_state = manager.info().unwrap().state;
    let start_stop_text = get_start_stop_text(&current_state);

    let main_tray_id = TrayId::new("main-tray");
    let icon = load_icon(icon_path);
    let mut tray_menu = Menu::new();
    let mut start_stop_item = tray_menu.add_item(MenuItemAttributes::new(start_stop_text));
    let restart_item = tray_menu.add_item(MenuItemAttributes::new("Restart"));
    let quit_item = tray_menu.add_item(MenuItemAttributes::new("Quit"));

    let mut system_tray = Some(
        SystemTrayBuilder::new(icon, Some(tray_menu))
            .with_id(main_tray_id)
            .with_tooltip("tao - windowing creation library")
            .build(&event_loop)
            .unwrap(),
    );
    #[cfg(target_os = "macos")]
    if let Some(t) = system_tray.as_mut() {
        t.set_title("Tao")
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            Instant::now()
                .checked_add(Duration::from_millis(1000))
                .unwrap(),
        );

        let current_state = manager.info().unwrap().state;
        start_stop_item.set_title(get_start_stop_text(&current_state));
        if let Event::MenuEvent {
            menu_id,
            origin: MenuType::ContextMenu,
            ..
        } = event
        {
            if menu_id == quit_item.clone().id() {
                system_tray.take();
                *control_flow = ControlFlow::Exit;
            } else if menu_id == start_stop_item.clone().id() {
                if current_state == State::Started {
                    manager.stop().unwrap();
                } else {
                    manager.start().unwrap();
                }
            } else if menu_id == restart_item.clone().id() {
                manager.restart().unwrap();
            }
        }
    });
}

fn get_start_stop_text(state: &State) -> &str {
    if state == &State::Started {
        "Stop"
    } else {
        "Start"
    }
}

fn load_icon(path: &std::path::Path) -> tao::system_tray::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tao::system_tray::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon")
}
