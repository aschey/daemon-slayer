use std::thread;

use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};

fn main() {
    let tray_start = CustomMenuItem::new("start".to_string(), "Start");
    let tray_stop = CustomMenuItem::new("stop".to_string(), "Stop");
    let tray_quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(tray_quit)
        .add_item(tray_start)
        .add_item(tray_stop);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| {
            on_tray_event(app, &event);
            let win = app.get_window("main").unwrap();
            if let SystemTrayEvent::LeftClick { position, size, .. } = event {
                if win.is_visible().unwrap() {
                    win.hide();
                } else {
                    win.move_window(Position::TrayCenter);
                    win.show();
                }
            }
            if let SystemTrayEvent::MenuItemClick { id, .. } = event {
                match id.as_str() {
                    "quit" => {
                        app.exit(0);
                    }
                    // "tray_bottom_left" => win.move_window(Position::TrayBottomLeft),
                    // "tray_right" => win.move_window(Position::TrayRight),
                    // "tray_bottom_right" => win.move_window(Position::TrayBottomRight),
                    // "tray_center" => win.move_window(Position::TrayCenter),
                    // "tray_bottom_center" => win.move_window(Position::TrayBottomCenter),
                    _ => {}
                };
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
