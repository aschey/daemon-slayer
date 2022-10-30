use std::{env::args, os, sync::Arc, time::Duration};

use daemon_slayer_client::{Level, Manager, ServiceManager, State as ServiceState};
use tauri::{
    api, tauri_build_context, CustomMenuItem, Manager as TauriManager, RunEvent, State, SystemTray,
    SystemTrayEvent, SystemTrayMenu, WindowBuilder, WindowEvent, WindowUrl,
};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};
use tracing_ipc::run_ipc_client;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    service_state: String,
}

#[derive(Clone)]
struct ManagerWrapper {
    manager: ServiceManager,
}

impl ManagerWrapper {
    fn get_service_state(&self) -> String {
        let state = self.manager.info().unwrap().state;
        match state {
            ServiceState::Started => "started".to_string(),
            ServiceState::Stopped => "stopped".to_string(),
            ServiceState::NotInstalled => "not_installed".to_string(),
        }
    }

    fn get_start_stop_text(&self) -> &str {
        if self.get_service_state() == "started" {
            "Stop"
        } else {
            "Start"
        }
    }

    fn toggle(&self) {
        if self.get_service_state() == "started" {
            self.manager.stop().unwrap();
        } else {
            self.manager.start().unwrap();
        }
    }

    fn restart(&self) {
        self.manager.restart().unwrap();
    }
}

fn main() {
    let manager = ManagerWrapper {
        manager: ServiceManager::builder(args().nth(1).unwrap())
            .with_service_level(Level::User)
            .build()
            .unwrap(),
    };

    let tray_open = CustomMenuItem::new("open".to_string(), "Open");
    let tray_start_stop =
        CustomMenuItem::new("start_stop".to_string(), manager.get_start_stop_text());
    let tray_restart = CustomMenuItem::new("restart".to_string(), "Restart");
    let tray_quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(tray_open)
        .add_item(tray_start_stop)
        .add_item(tray_restart)
        .add_item(tray_quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);
    let manager_ = manager.clone();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![toggle, restart, get_service_state])
        .plugin(tauri_plugin_positioner::init())
        .system_tray(system_tray)
        .setup(move |app| {
            let win = app.get_window("main").unwrap_or_else(|| {
                WindowBuilder::new(
                    &app.app_handle(),
                    "main",
                    WindowUrl::App("index.html".into()),
                )
                .always_on_top(true)
                .inner_size(800.0, 600.0)
                .title("daemon-slayer-gui")
                .visible(false)
                .skip_taskbar(true)
                .build()
                .unwrap()
            });
            let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(32);
            tauri::async_runtime::spawn(async move {
                run_ipc_client("daemon_slayer_axum", log_tx).await;
            });
            let win_ = win.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(log) = log_rx.recv().await {
                    win_.emit("log", log).unwrap();
                }
            });
            tauri::async_runtime::spawn(async move {
                let mut state = manager_.get_service_state();
                loop {
                    let new_state = manager_.get_service_state();
                    if new_state != state {
                        state = new_state;
                        win.emit(
                            "service_state",
                            Payload {
                                service_state: state.clone(),
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
            #[cfg(not(target_os = "linux"))]
            if let WindowEvent::Focused(false) = event.event() {
                event.window().hide().unwrap();
            }
            if let WindowEvent::CloseRequested { api, .. } = event.event() {
                api.prevent_close();
                event.window().hide().unwrap();
            }
        })
        .on_system_tray_event(move |app, event| {
            app.tray_handle()
                .get_item("start_stop")
                .set_title(manager.get_start_stop_text())
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
                    "open" => {
                        #[cfg(not(target_os = "linux"))]
                        win.move_window(Position::TrayCenter).unwrap();
                        win.show().unwrap();
                    }
                    "start_stop" => {
                        manager.toggle();
                    }
                    "restart" => {
                        manager.restart();
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
fn toggle(manager: State<ManagerWrapper>) {
    manager.toggle();
}

#[tauri::command]
fn restart(manager: State<ManagerWrapper>) {
    manager.restart();
}

#[tauri::command]
fn get_service_state(manager: State<ManagerWrapper>) -> String {
    manager.get_service_state()
}
