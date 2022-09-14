use std::error::Error;

use tracing::{error, info};

use crate::{EventHandlerAsync, HandlerAsync, HandlerSync, ServiceAsync};

#[cfg(feature = "async-tokio")]
pub fn get_service_main_async<T: crate::HandlerAsync + Send>() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(get_service_main_impl_async::<T>())
}

#[cfg(feature = "blocking")]
pub fn get_service_main_sync<T: crate::HandlerSync + Send>() {
    get_service_main_impl_sync::<T>()
}

#[maybe_async_cfg::maybe(
    idents(
        Handler,
        get_channel(snake),
        start_file_watcher(snake),
        start_event_loop(snake),
        send_stop_signal(snake),
        run_service_main(snake)
    ),
    sync(feature = "blocking", drop_attrs(cfg)),
    async(feature = "async-tokio")
)]
async fn get_service_main_impl<T: crate::Handler + Send>() {
    let mut handler = T::new();
    let event_handler = handler.get_event_handler();

    let (event_tx, event_rx) = get_channel();

    let _file_watcher = start_file_watcher(handler.get_watch_paths(), event_tx.clone());

    start_event_loop(event_rx, event_handler);

    let windows_service_event_handler = move |control_event| -> crate::windows_service::service_control_handler::ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            crate::windows_service::service::ServiceControl::Interrogate => crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError,

            // Handle stop
            crate::windows_service::service::ServiceControl::Stop => {
                if let Err(e) = send_stop_signal(&event_tx) {
                    error!("Error sending stop signal: {e:?}");
                }
                crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError
            }

            _ => crate::windows_service::service_control_handler::ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match crate::windows_service::service_control_handler::register(
        T::get_service_name(),
        windows_service_event_handler,
    ) {
        Ok(handle) => std::sync::Arc::new(std::sync::Mutex::new(handle)),
        Err(e) => {
            error!("Error registering control handler {e}");
            return;
        }
    };
    let status_handle_ = status_handle.clone();
    let on_started = move || {
        status_handle_
            .lock()
            .unwrap()
            .set_service_status(crate::windows_service::service::ServiceStatus {
                service_type: crate::windows_service::service::ServiceType::OWN_PROCESS,
                current_state: crate::windows_service::service::ServiceState::Running,
                controls_accepted: crate::windows_service::service::ServiceControlAccept::STOP,
                exit_code: crate::windows_service::service::ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: std::time::Duration::default(),
                process_id: None,
            })
            .unwrap();
    };

    let service_result = handler.run_service(on_started).await;

    let exit_code = match service_result {
        Ok(()) => 0,
        Err(_) => 1,
    };
    status_handle
        .lock()
        .unwrap()
        .set_service_status(crate::windows_service::service::ServiceStatus {
            service_type: crate::windows_service::service::ServiceType::OWN_PROCESS,
            current_state: crate::windows_service::service::ServiceState::Stopped,
            controls_accepted: crate::windows_service::service::ServiceControlAccept::empty(),
            exit_code: crate::windows_service::service::ServiceExitCode::Win32(exit_code),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })
        .unwrap();
}

#[cfg(feature = "async-tokio")]
fn get_channel_async() -> (
    tokio::sync::mpsc::Sender<crate::Event>,
    tokio::sync::mpsc::Receiver<crate::Event>,
) {
    tokio::sync::mpsc::channel(32)
}

fn get_channel_sync() -> (
    std::sync::mpsc::Sender<crate::Event>,
    std::sync::mpsc::Receiver<crate::Event>,
) {
    std::sync::mpsc::channel()
}

#[cfg(feature = "async-tokio")]
fn send_stop_signal_async(
    event_tx: &tokio::sync::mpsc::Sender<crate::Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    futures::executor::block_on(async {
        event_tx
            .send(crate::Event::SignalReceived(crate::Signal::SIGINT))
            .await
            .unwrap();
    });

    Ok(())
}

#[cfg(feature = "blocking")]
fn send_stop_signal_sync(
    event_tx: &std::sync::mpsc::Sender<crate::Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    event_tx.send(crate::Event::SignalReceived(crate::Signal::SIGINT))?;
    Ok(())
}

#[cfg(feature = "blocking")]
fn start_file_watcher_sync(
    paths: Vec<std::path::PathBuf>,
    tx: std::sync::mpsc::Sender<crate::Event>,
) -> Result<
    notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    Box<dyn Error + Send + Sync>,
> {
    let (watch_tx, watch_rx) = std::sync::mpsc::channel();
    let mut debouncer = crate::notify_debouncer_mini::new_debouncer(
        std::time::Duration::from_secs(2),
        None,
        watch_tx,
    )?;
    let watcher = debouncer.watcher();

    for path in paths.iter() {
        watcher
            .watch(path, crate::notify::RecursiveMode::Recursive)
            .unwrap();
    }

    std::thread::spawn(move || {
        for events in watch_rx {
            let e = events.unwrap().into_iter().map(|e| e.path).collect();
            tx.send(crate::Event::FileChanged(e)).unwrap();
        }
    });

    Ok(debouncer)
}

#[cfg(feature = "async-tokio")]
fn start_file_watcher_async(
    paths: Vec<std::path::PathBuf>,
    tx: crate::tokio::sync::mpsc::Sender<crate::Event>,
) -> Result<
    notify_debouncer_mini::Debouncer<crate::notify::RecommendedWatcher>,
    Box<dyn Error + Send + Sync>,
> {
    if paths.is_empty() {
        info!("Not starting file watcher because there are no files configured");
    }
    let (watch_tx, watch_rx) = std::sync::mpsc::channel();
    let mut debouncer = crate::notify_debouncer_mini::new_debouncer(
        std::time::Duration::from_secs(2),
        None,
        watch_tx,
    )?;
    let watcher = debouncer.watcher();

    for path in paths.iter() {
        match watcher.watch(path, crate::notify::RecursiveMode::Recursive) {
            Ok(_) => {
                info!("Watching {path:?}");
            }
            Err(e) => {
                error!("Error watching {path:?}: {e:?}");
            }
        }
    }

    tokio::task::spawn_blocking(move || {
        info!("Starting file watcher task");
        for events in watch_rx {
            info!("Got file event");
            let e = events.unwrap().into_iter().map(|e| e.path).collect();
            futures::executor::block_on(async {
                tx.send(crate::Event::FileChanged(e)).await.unwrap();
            });
        }
    });

    Ok(debouncer)
}

#[cfg(feature = "async-tokio")]
fn start_event_loop_async(
    mut event_rx: tokio::sync::mpsc::Receiver<crate::Event>,
    event_handler: EventHandlerAsync,
) {
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            info!("received event");
            event_handler(event).await.unwrap();
        }
    });
}

#[cfg(feature = "blocking")]
fn start_event_loop_sync(
    event_rx: std::sync::mpsc::Receiver<crate::Event>,
    event_handler: crate::EventHandlerSync,
) {
    std::thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            event_handler(event).unwrap();
        }
    });
}

#[cfg(feature = "async-tokio")]
pub async fn get_direct_handler_async(
    mut handler: impl HandlerAsync + Send,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(feature = "direct")]
    {
        let event_handler = handler.get_event_handler();
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(32);
        let _file_watcher = start_file_watcher_async(handler.get_watch_paths(), event_tx);
        let handle: crate::tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
            crate::tokio::spawn(async move {
                let mut ctrl_c = tokio::signal::windows::ctrl_c()?;
                loop {
                    crate::tokio::select! {
                        stop_event = ctrl_c.recv() => {

                            match stop_event {
                                Some(_) => {
                                    event_handler(crate::Event::SignalReceived(crate::Signal::SIGINT)).await?;
                                   // return Ok(());
                                }
                                None => {
                                   //return Ok(());
                                }
                            }
                        }
                        file_event = event_rx.recv() => {
                            match file_event {
                                Some(file_event) => {
                                    info!("Received file event");
                                    event_handler(file_event).await?;
                                },
                                None => {
                                   // return Ok(());
                                }
                            }
                        }

                    }
                }
            });
        handler.run_service(|| {}).await?;
        //handle.await?;
    }
    Ok(())
}

#[cfg(feature = "blocking")]
pub fn get_direct_handler_sync(
    mut handler: impl HandlerSync + Send,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(feature = "direct")]
    {
        let event_handler = handler.get_event_handler();
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let _file_watcher = start_file_watcher_sync(handler.get_watch_paths(), event_tx);
        let handle: std::thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
            std::thread::spawn(move || {
                let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

                signal_hook::flag::register(
                    signal_hook::consts::SIGINT,
                    std::sync::Arc::clone(&term),
                )?;

                while !term.load(std::sync::atomic::Ordering::Relaxed) {
                    if let Ok(event) = event_rx.try_recv() {
                        event_handler(event)?;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                event_handler(crate::Event::SignalReceived(crate::Signal::SIGINT))?;
                Ok(())
            });

        handler.run_service(|| {})?;
        handle.join().unwrap()?;
    }

    Ok(())
}
