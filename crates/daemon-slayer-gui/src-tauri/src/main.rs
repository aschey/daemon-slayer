use std::sync::RwLock;
use std::{env::args, os, sync::Arc, time::Duration};

use daemon_slayer_client::configuration::Level;
use daemon_slayer_client::{Info, Manager};
use daemon_slayer_health_check::HealthCheck;
use daemon_slayer_health_check::{HttpHealthCheck, HttpRequestType};
use tauri::{
    api, tauri_build_context, CustomMenuItem, Manager as TauriManager, RunEvent, State, SystemTray,
    SystemTrayEvent, SystemTrayMenu, WindowBuilder, WindowEvent, WindowUrl,
};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};
use tracing_ipc::run_ipc_client;

#[derive(Clone)]
struct ManagerWrapper {
    manager: Box<dyn Manager>,
}

impl ManagerWrapper {
    fn get_service_info(&self) -> Info {
        self.manager.info().unwrap()
    }

    fn get_start_stop_text(&self) -> &str {
        if self.get_service_info().state == daemon_slayer_client::State::Started {
            "Stop"
        } else {
            "Start"
        }
    }

    fn toggle_start_stop(&self) {
        if self.get_service_info().state == daemon_slayer_client::State::Started {
            self.manager.stop().unwrap();
        } else {
            self.manager.start().unwrap();
        }
    }

    fn toggle_enable_disable(&mut self) {
        if self.get_service_info().autostart == Some(true) {
            self.manager.enable_autostart().unwrap();
        } else {
            self.manager.disable_autostart().unwrap();
        }
    }

    fn restart(&self) {
        self.manager.restart().unwrap();
    }
}

fn main() {
    let manager = Arc::new(RwLock::new(ManagerWrapper {
        manager: daemon_slayer_client::builder(args().nth(1).unwrap().parse().unwrap())
            .with_service_level(if cfg!(windows) {
                Level::System
            } else {
                Level::User
            })
            .build()
            .unwrap(),
    }));

    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("status", "").disabled())
        .add_item(CustomMenuItem::new("open".to_string(), "Open"))
        .add_item(CustomMenuItem::new(
            "start_stop".to_string(),
            manager.read().unwrap().get_start_stop_text(),
        ))
        .add_item(CustomMenuItem::new("restart".to_string(), "Restart"))
        .add_item(CustomMenuItem::new("quit".to_string(), "Quit"));

    let system_tray = SystemTray::new().with_menu(tray_menu);
    let manager_ = manager.clone();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            toggle_start_stop,
            toggle_enable_disable,
            restart,
            get_service_info
        ])
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
            let tray_handle = app.tray_handle();
            let status_handle = tray_handle.get_item("status");
            tauri::async_runtime::spawn(async move {
                let mut info = manager_.read().unwrap().get_service_info();
                let mut health_check =
                    HttpHealthCheck::new(HttpRequestType::Get, "http://127.0.0.1:3000/health")
                        .unwrap();
                loop {
                    let new_info = manager_.read().unwrap().get_service_info();
                    if new_info != info {
                        info = new_info;
                        win.emit("service_info", info.clone()).unwrap();
                    }
                    if info.state == daemon_slayer_client::State::Started {
                        match health_check.invoke().await {
                            Ok(_) => {
                                win.emit("healthy", true).unwrap();
                                status_handle.set_title("✓ Healthy").unwrap();
                            }
                            Err(_) => {
                                win.emit("healthy", false).unwrap();
                                status_handle.set_title("✕ Unhealthy").unwrap();
                            }
                        };
                    } else {
                        status_handle.set_title("■ Stopped").unwrap();
                    }

                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });
            Ok(())
        })
        .manage(manager.clone())
        .on_window_event(|event| {
            if let WindowEvent::CloseRequested { api, .. } = event.event() {
                api.prevent_close();
                event.window().hide().unwrap();
            }
        })
        .on_system_tray_event(move |app, event| {
            app.tray_handle()
                .get_item("start_stop")
                .set_title(manager.read().unwrap().get_start_stop_text())
                .unwrap();
            on_tray_event(app, &event);
            let win = app.get_window("main").unwrap();

            if let SystemTrayEvent::LeftClick { .. } = event {
                let size = win.inner_size().unwrap();

                if win.is_visible().unwrap() && size.width > 0 && size.height > 0 {
                    win.hide().unwrap();
                } else {
                    win.unminimize().unwrap();
                    win.show().unwrap();
                }
            }
            if let SystemTrayEvent::MenuItemClick { id, .. } = event {
                match id.as_str() {
                    "open" => {
                        win.unminimize().unwrap();
                        win.show().unwrap();
                    }
                    "start_stop" => {
                        manager.read().unwrap().toggle_start_stop();
                    }
                    "restart" => {
                        manager.read().unwrap().restart();
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
fn toggle_start_stop(manager: State<Arc<RwLock<ManagerWrapper>>>) {
    manager.read().unwrap().toggle_start_stop();
}

#[tauri::command]
fn toggle_enable_disable(manager: State<Arc<RwLock<ManagerWrapper>>>) {
    manager.write().unwrap().toggle_enable_disable();
}

#[tauri::command]
fn restart(manager: State<Arc<RwLock<ManagerWrapper>>>) {
    manager.read().unwrap().restart();
}

#[tauri::command]
fn get_service_info(manager: State<Arc<RwLock<ManagerWrapper>>>) -> daemon_slayer_client::Info {
    manager.read().unwrap().get_service_info()
}
