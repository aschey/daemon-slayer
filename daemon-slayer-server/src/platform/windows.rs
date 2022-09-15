use std::error::Error;

use tracing::{error, info};

#[cfg(feature = "async-tokio")]
pub fn get_service_main_async<T: crate::HandlerAsync + Send>() {
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime failed to initialize");
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
        run_service_main(snake),
        join_handle(snake)
    ),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
async fn get_service_main_impl<T: crate::Handler + Send>() {
    let mut handler = T::new();
    let event_handler = handler.get_event_handler();

    let (event_tx, event_rx) = get_channel();

    #[cfg(feature = "file-watcher")]
    let file_task = match start_file_watcher(handler.get_watch_paths(), event_tx.clone()) {
        Ok((watcher, handle)) => (Some(watcher), Some(handle)),
        Err(e) => {
            error!("Error starting file watcher: {e}");
            (None, None)
        }
    };

    let event_handle = start_event_loop(event_rx, event_handler);

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
        if let Err(e) = status_handle_.lock().unwrap().set_service_status(
            crate::windows_service::service::ServiceStatus {
                service_type: crate::windows_service::service::ServiceType::OWN_PROCESS,
                current_state: crate::windows_service::service::ServiceState::Running,
                controls_accepted: crate::windows_service::service::ServiceControlAccept::STOP,
                exit_code: crate::windows_service::service::ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: std::time::Duration::default(),
                process_id: None,
            },
        ) {
            error!("Error setting status to 'running': {e:?}");
        }
    };

    let service_result = handler.run_service(on_started).await;

    let exit_code = match service_result {
        Ok(()) => 0,
        Err(e) => {
            error!("Service exited with error: {e}");
            1
        }
    };
    #[cfg(feature = "file-watcher")]
    if let (Some(file_watcher), Some(file_task_handle)) = file_task {
        drop(file_watcher);
        join_handle(file_task_handle).await;
    }

    {
        let handle = status_handle.lock().unwrap();
        if let Err(e) = handle.set_service_status(crate::windows_service::service::ServiceStatus {
            service_type: crate::windows_service::service::ServiceType::OWN_PROCESS,
            current_state: crate::windows_service::service::ServiceState::Stopped,
            controls_accepted: crate::windows_service::service::ServiceControlAccept::empty(),
            exit_code: crate::windows_service::service::ServiceExitCode::Win32(exit_code),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        }) {
            error!("Error setting status to stopped: {e:?}");
        }
    }

    drop(status_handle);
    join_handle(event_handle).await;
}

#[cfg(feature = "async-tokio")]
fn get_channel_async() -> (
    tokio::sync::mpsc::Sender<crate::Event>,
    tokio::sync::mpsc::Receiver<crate::Event>,
) {
    tokio::sync::mpsc::channel(32)
}

#[cfg(feature = "blocking")]
fn get_channel_sync() -> (
    std::sync::mpsc::Sender<crate::Event>,
    std::sync::mpsc::Receiver<crate::Event>,
) {
    std::sync::mpsc::channel()
}

#[cfg(feature = "async-tokio")]
async fn join_handle_async(
    handle: tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>,
) {
    if let Err(e) = handle.await {
        error!("Error joining task: {e:?}");
    }
}

#[cfg(feature = "blocking")]
fn join_handle_sync(handle: std::thread::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>) {
    if let Err(e) = handle.join() {
        error!("Error joining task: {e:?}");
    }
}

#[cfg(feature = "async-tokio")]
fn send_stop_signal_async(
    event_tx: &tokio::sync::mpsc::Sender<crate::Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    futures::executor::block_on(async {
        event_tx
            .send(crate::Event::SignalReceived(crate::Signal::SIGINT))
            .await
    })?;

    Ok(())
}

#[cfg(feature = "blocking")]
fn send_stop_signal_sync(
    event_tx: &std::sync::mpsc::Sender<crate::Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    event_tx.send(crate::Event::SignalReceived(crate::Signal::SIGINT))?;
    Ok(())
}

#[cfg(all(feature = "blocking", feature = "file-watcher"))]
fn start_file_watcher_sync(
    paths: Vec<std::path::PathBuf>,
    tx: std::sync::mpsc::Sender<crate::Event>,
) -> Result<
    (
        notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
        std::thread::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>,
    ),
    Box<dyn Error + Send + Sync>,
> {
    let (watch_tx, watch_rx) = std::sync::mpsc::channel();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(std::time::Duration::from_secs(2), None, watch_tx)?;
    let watcher = debouncer.watcher();

    for path in paths.iter() {
        if let Err(e) = watcher.watch(path, notify::RecursiveMode::Recursive) {
            error!("Error watching path: {e:?}");
        }
    }

    let handle: std::thread::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
        std::thread::spawn(move || {
            for events in watch_rx {
                let e = events.unwrap().into_iter().map(|e| e.path).collect();
                tx.send(crate::Event::FileChanged(e))?;
            }
            Ok(())
        });

    Ok((debouncer, handle))
}

#[cfg(all(feature = "async-tokio", feature = "file-watcher"))]
fn start_file_watcher_async(
    paths: Vec<std::path::PathBuf>,
    tx: crate::tokio::sync::mpsc::Sender<crate::Event>,
) -> Result<
    (
        notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
        tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>,
    ),
    Box<dyn Error + Send + Sync>,
> {
    if paths.is_empty() {
        info!("Not starting file watcher because there are no files configured");
    }
    let (watch_tx, watch_rx) = std::sync::mpsc::channel();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(std::time::Duration::from_secs(2), None, watch_tx)?;
    let watcher = debouncer.watcher();

    for path in paths.iter() {
        match watcher.watch(path, notify::RecursiveMode::Recursive) {
            Ok(_) => {
                info!("Watching {path:?}");
            }
            Err(e) => {
                error!("Error watching {path:?}: {e:?}");
            }
        }
    }

    let handle = tokio::task::spawn_blocking(move || {
        info!("Starting file watcher task");
        for events in watch_rx {
            info!("Got file event");
            let e = events.unwrap().into_iter().map(|e| e.path).collect();
            let handle: Result<(), Box<dyn Error + Send + Sync>> =
                futures::executor::block_on(async {
                    tx.send(crate::Event::FileChanged(e)).await?;
                    Ok(())
                });
            handle?;
        }
        Ok(())
    });

    Ok((debouncer, handle))
}

#[cfg(feature = "async-tokio")]
fn start_event_loop_async(
    mut event_rx: tokio::sync::mpsc::Receiver<crate::Event>,
    event_handler: crate::EventHandlerAsync,
) -> tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            info!("received event");
            event_handler(event).await?;
        }
        Ok(())
    })
}

#[cfg(feature = "blocking")]
fn start_event_loop_sync(
    event_rx: std::sync::mpsc::Receiver<crate::Event>,
    event_handler: crate::EventHandlerSync,
) -> std::thread::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    std::thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            event_handler(event)?;
        }
        Ok(())
    })
}

#[cfg(feature = "async-tokio")]
pub async fn get_direct_handler_async(
    mut handler: impl crate::HandlerAsync + Send,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(feature = "direct")]
    {
        let event_handler = handler.get_event_handler();
        #[cfg(feature = "file-watcher")]
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(32);
        #[cfg(not(feature = "file-watcher"))]
        let (_event_tx, mut event_rx) = tokio::sync::mpsc::channel(32);
        #[cfg(feature = "file-watcher")]
        let _file_watcher = start_file_watcher_async(handler.get_watch_paths(), event_tx);
        let handle: crate::tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
            crate::tokio::spawn(async move {
                let mut ctrl_c = tokio::signal::windows::ctrl_c()?;
                loop {
                    crate::tokio::select! {
                        stop_event = ctrl_c.recv() => {
                            if let Some(()) =  stop_event {
                                event_handler(crate::Event::SignalReceived(crate::Signal::SIGINT)).await?;
                                return Ok(());
                            }
                        }
                        file_event = event_rx.recv() => {
                            if let Some(file_event) = file_event  {
                                info!("Received file event");
                                event_handler(file_event).await?;
                            }
                        }
                    }
                }
            });
        handler.run_service(|| {}).await?;
        handle.await??;
    }
    Ok(())
}

#[cfg(feature = "blocking")]
pub fn get_direct_handler_sync(
    mut handler: impl crate::HandlerSync + Send,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(feature = "direct")]
    {
        let event_handler = handler.get_event_handler();
        #[cfg(feature = "file-watcher")]
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        #[cfg(feature = "file-watcher")]
        let _file_watcher = start_file_watcher_sync(handler.get_watch_paths(), event_tx);
        let handle: std::thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
            std::thread::spawn(move || {
                let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

                signal_hook::flag::register(
                    signal_hook::consts::SIGINT,
                    std::sync::Arc::clone(&term),
                )?;

                while !term.load(std::sync::atomic::Ordering::Relaxed) {
                    #[cfg(feature = "file-watcher")]
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
