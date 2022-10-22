use std::{sync::Arc, time::Duration};

use daemon_slayer_client::{Manager, ServiceManager, State as ServiceState};
use tauri::{
    CustomMenuItem, Manager as TauriManager, State, SystemTray, SystemTrayEvent, SystemTrayMenu,
    WindowEvent,
};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    service_state: String,
}

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
    let manager_ = manager.clone();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![toggle, get_service_state])
        .plugin(tauri_plugin_positioner::init())
        .system_tray(system_tray)
        .setup(move |app| {
            let win = app.get_window("main").unwrap();
            tauri::async_runtime::spawn(async move {
                let mut state = manager_.info().unwrap().state;
                loop {
                    let new_state = manager_.info().unwrap().state;
                    if new_state != state {
                        state = new_state;
                        win.emit(
                            "service_state",
                            Payload {
                                service_state: match state {
                                    ServiceState::Started => "started".to_string(),
                                    ServiceState::Stopped => "stopped".to_string(),
                                    ServiceState::NotInstalled => "not_installed".to_string(),
                                },
                            },
                        )
                        .unwrap();
                    }

                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });
            Ok(())
        })
        .manage(manager.clone())
        .on_window_event(|event| {
            if let WindowEvent::Focused(false) = event.event() {
                event.window().hide().unwrap();
            }
        })
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
                        if state == ServiceState::Started {
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

#[tauri::command]
fn toggle(state: String, manager: State<ServiceManager>) {
    if state == "started" {
        manager.stop().unwrap();
    } else {
        manager.start().unwrap();
    }
}

#[tauri::command]
fn get_service_state(manager: State<ServiceManager>) -> String {
    let state = manager.info().unwrap().state;
    match state {
        ServiceState::Started => "started".to_string(),
        ServiceState::Stopped => "stopped".to_string(),
        ServiceState::NotInstalled => "not_installed".to_string(),
    }
}

fn get_start_stop_text(state: &ServiceState) -> &str {
    if state == &ServiceState::Started {
        "Stop"
    } else {
        "Start"
    }
}
