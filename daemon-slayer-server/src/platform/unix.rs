use std::{error::Error, time::Duration};
use tracing::{error, info};

#[cfg(feature = "async-tokio")]
pub async fn run_service_main_async<T: crate::HandlerAsync + Send>(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handler = T::new();
    let event_handler = handler.get_event_handler();
    let mut config = handler.configure(crate::ServiceConfig::default());

    let signals = crate::signal_hook_tokio::Signals::new(&[
        crate::signal_hook::consts::signal::SIGHUP,
        crate::signal_hook::consts::signal::SIGTERM,
        crate::signal_hook::consts::signal::SIGINT,
        crate::signal_hook::consts::signal::SIGQUIT,
        crate::signal_hook::consts::signal::SIGTSTP,
        crate::signal_hook::consts::signal::SIGCHLD,
        crate::signal_hook::consts::signal::SIGCONT,
    ])
    .unwrap();

    let (event_tx, mut event_rx) = crate::tokio::sync::mpsc::channel(32);

    #[cfg(feature = "file-watcher")]
    let debouncer = start_file_watcher_async(&config, event_tx.clone());

    let context = get_context_async(&mut config, event_tx).await;
    let signals_handle = signals.handle();

    let event_task: crate::tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
        crate::tokio::spawn(async move {
            use crate::futures::stream::StreamExt;

            let mut signals = signals.fuse();

            loop {
                crate::tokio::select! {
                    event = event_rx.recv() => {
                        match event {
                            Some(event) => {
                                event_handler(event).await?;
                            },
                            None => {
                                return Ok(());
                            }
                        }
                    }
                    signal = signals.next() => {
                        match signal {
                            Some(signal) => {
                                #[cfg(target_os = "linux")]
                                crate::sd_notify::notify(false, &[crate::sd_notify::NotifyState::Stopping]).unwrap();
                                let signal_name = crate::signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
                                event_handler(crate::Event::SignalReceived(signal_name.into())).await?;
                            },
                            None => {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        });

    let result = handler
        .run_service(context, || {
            #[cfg(target_os = "linux")]
            crate::sd_notify::notify(false, &[crate::sd_notify::NotifyState::Ready]).unwrap();
        })
        .await;

    signals_handle.close();
    #[cfg(feature = "file-watcher")]
    drop(debouncer);
    event_task.await.unwrap()?;
    result
}

#[cfg(feature = "blocking")]
pub fn run_service_main_sync<T: crate::HandlerSync + Send>(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handler = T::new();
    let event_handler = handler.get_event_handler();
    let mut config = handler.configure(crate::ServiceConfig::default());

    let mut signals = crate::signal_hook::iterator::Signals::new(&[
        crate::signal_hook::consts::signal::SIGHUP,
        crate::signal_hook::consts::signal::SIGTERM,
        crate::signal_hook::consts::signal::SIGINT,
        crate::signal_hook::consts::signal::SIGQUIT,
        crate::signal_hook::consts::signal::SIGTSTP,
        crate::signal_hook::consts::signal::SIGCHLD,
        crate::signal_hook::consts::signal::SIGCONT,
    ])
    .unwrap();

    let (event_tx, event_rx) = std::sync::mpsc::channel();

    #[cfg(feature = "file-watcher")]
    let _debouncer = start_file_watcher_sync(&config, event_tx.clone());
    let context = get_context_sync(&mut config, event_tx);
    let signals_handle = signals.handle();
    let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let term_ = term.clone();
    let handle: std::thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
        std::thread::spawn(move || loop {
            if term_.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(());
            }
            let mut wait = true;
            for signal in signals.pending() {
                #[cfg(target_os = "linux")]
                crate::sd_notify::notify(false, &[crate::sd_notify::NotifyState::Stopping])
                    .unwrap();
                let signal_name =
                    crate::signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
                event_handler(crate::Event::SignalReceived(signal_name.into()))?;
                wait = false;
            }
            if let Ok(event) = event_rx.try_recv() {
                event_handler(event)?;
                wait = false;
            }
            if wait {
                std::thread::sleep(Duration::from_millis(10));
            }
        });

    handler.run_service(context, || {
        #[cfg(target_os = "linux")]
        crate::sd_notify::notify(false, &[crate::sd_notify::NotifyState::Ready]).unwrap();
    })?;

    signals_handle.close();
    term.store(true, std::sync::atomic::Ordering::Relaxed);
    handle.join().unwrap()?;

    Ok(())
}

#[cfg(all(feature = "async-tokio", feature = "file-watcher"))]
fn start_file_watcher_async(
    config: &crate::ServiceConfig,
    tx: crate::tokio::sync::mpsc::Sender<crate::Event>,
) -> Result<
    (
        notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
        tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>,
    ),
    Box<dyn Error + Send + Sync>,
> {
    if config.watch_paths.is_empty() {
        info!("Not starting file watcher because there are no files configured");
    }
    let (watch_tx, watch_rx) = std::sync::mpsc::channel();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(std::time::Duration::from_secs(2), None, watch_tx)?;
    let watcher = debouncer.watcher();

    for path in config.watch_paths.iter() {
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

#[cfg(all(feature = "blocking", feature = "file-watcher"))]
fn start_file_watcher_sync(
    config: &crate::ServiceConfig,
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

    for path in config.watch_paths.iter() {
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

#[cfg(feature = "task-queue")]
async fn get_context_async(
    config: &mut crate::ServiceConfig,
    tx: tokio::sync::mpsc::Sender<crate::Event>,
) -> crate::ServiceContextAsync {
    let task_queue = config.task_queue_builder.take().unwrap().build().await;
    let mut event_rx = task_queue.subscribe_events();
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if let Err(e) = tx.send(crate::Event::TaskQueueEvent(event)).await {
                error!("Error sending task queue event: {e:?}");
            }
        }
    });
    crate::ServiceContextAsync { task_queue }
}

#[cfg(all(not(feature = "task-queue"), feature = "async-tokio"))]
async fn get_context_async(
    config: &mut crate::ServiceConfig,
    tx: tokio::sync::mpsc::Sender<crate::Event>,
) -> crate::ServiceContextAsync {
    crate::ServiceContextAsync {}
}

#[cfg(feature = "blocking")]
fn get_context_sync(
    _config: &mut crate::ServiceConfig,
    _tx: std::sync::mpsc::Sender<crate::Event>,
) -> crate::ServiceContextSync {
    crate::ServiceContextSync {}
}
