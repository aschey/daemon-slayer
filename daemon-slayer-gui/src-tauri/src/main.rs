use daemon_slayer_client::{Manager, ServiceManager, State};
use tauri::{CustomMenuItem, Manager as TauriManager, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};

fn main() {
    let manager = ServiceManager::new("daemon_slayer_axum").unwrap();
    let state = manager.info().unwrap().state;
    let tray_start_stop =
        CustomMenuItem::new("start_stop".to_string(), get_start_stop_text(&state));
    let tray_restart = CustomMenuItem::new("restart".to_string(), "Restart");
    let tray_quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(tray_start_stop)
        .add_item(tray_restart)
        .add_item(tray_quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .system_tray(system_tray)
        .on_system_tray_event(move |app, event| {
            let state = manager.info().unwrap().state;
            app.tray_handle()
                .get_item("start_stop")
                .set_title(get_start_stop_text(&state))
                .unwrap();
            on_tray_event(app, &event);
            let win = app.get_window("main").unwrap();
            if let SystemTrayEvent::LeftClick { .. } = event {
                if win.is_visible().unwrap() {
                    win.hide().unwrap();
                } else {
                    win.move_window(Position::TrayCenter).unwrap();
                    win.show().unwrap();
                }
            }
            if let SystemTrayEvent::MenuItemClick { id, .. } = event {
                match id.as_str() {
                    "start_stop" => {
                        if state == State::Started {
                            manager.stop().unwrap();
                        } else {
                            manager.start().unwrap();
                        }
                    }
                    "restart" => {
                        manager.restart().unwrap();
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                };
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

fn get_start_stop_text(state: &State) -> &str {
    if state == &State::Started {
        "Stop"
    } else {
        "Start"
    }
}
