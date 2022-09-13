use std::{error::Error, time::Duration};

#[cfg(feature = "async-tokio")]
pub async fn run_service_main_async<T: crate::HandlerAsync + Send>(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::{EventHandlerAsync, HandlerAsync};
    let mut handler = T::new();
    let event_handler = handler.get_event_handler();

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

    let (file_tx, mut file_rx) = crate::tokio::sync::mpsc::channel(32);
    let debouncer = start_file_watcher_async(handler.get_watch_paths(), file_tx);

    let signals_handle = signals.handle();

    let event_task: crate::tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
        crate::tokio::spawn(async move {
            use crate::futures::stream::StreamExt;

            let mut signals = signals.fuse();

            loop {
                crate::tokio::select! {
                    files = file_rx.recv() => {
                        match files {
                            Some(files) => {
                                event_handler(files).await?;
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
        .run_service(|| {
            #[cfg(target_os = "linux")]
            crate::sd_notify::notify(false, &[crate::sd_notify::NotifyState::Ready]).unwrap();
        })
        .await;

    signals_handle.close();
    drop(debouncer);
    event_task.await.unwrap()?;
    result
}

#[cfg(feature = "blocking")]
pub fn run_service_main_sync<T: crate::HandlerSync + Send>(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::{EventHandlerSync, HandlerSync};
    let mut handler = T::new();
    let event_handler = handler.get_event_handler();

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

    let (file_tx, mut file_rx) = crate::tokio::sync::mpsc::channel(32);
    let debouncer = start_file_watcher_async(handler.get_watch_paths(), file_tx);
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
            if let Ok(files) = file_rx.try_recv() {
                event_handler(files)?;
                wait = false;
            }
            if wait {
                std::thread::sleep(Duration::from_millis(10));
            }
        });

    handler.run_service(|| {
        #[cfg(target_os = "linux")]
        crate::sd_notify::notify(false, &[crate::sd_notify::NotifyState::Ready]).unwrap();
    })?;
    signals_handle.close();
    term.store(true, std::sync::atomic::Ordering::Relaxed);
    handle.join().unwrap()?;

    Ok(())
}

#[cfg(feature = "async-tokio")]
fn start_file_watcher_async(
    paths: &[std::path::PathBuf],
    tx: crate::tokio::sync::mpsc::Sender<crate::Event>,
) -> crate::notify_debouncer_mini::Debouncer<crate::notify::RecommendedWatcher> {
    let (watch_tx, watch_rx) = std::sync::mpsc::channel();
    let mut debouncer = crate::notify_debouncer_mini::new_debouncer(
        std::time::Duration::from_secs(2),
        None,
        watch_tx,
    )
    .unwrap();
    let watcher = debouncer.watcher();

    for path in paths {
        watcher
            .watch(path, crate::notify::RecursiveMode::Recursive)
            .unwrap();
    }

    crate::tokio::task::spawn_blocking(move || {
        for events in watch_rx {
            let e = events.unwrap().into_iter().map(|e| e.path).collect();
            futures::executor::block_on(async {
                tx.send(crate::Event::FileChanged(e)).await.unwrap();
            });
        }
    });

    debouncer
}

#[cfg(feature = "blocking")]
fn start_file_watcher_sync(
    paths: &[std::path::PathBuf],
    tx: std::sync::mpsc::Sender<crate::Event>,
) -> crate::notify_debouncer_mini::Debouncer<crate::notify::RecommendedWatcher> {
    let (watch_tx, watch_rx) = std::sync::mpsc::channel();
    let mut debouncer = crate::notify_debouncer_mini::new_debouncer(
        std::time::Duration::from_secs(2),
        None,
        watch_tx,
    )
    .unwrap();
    let watcher = debouncer.watcher();

    for path in paths {
        watcher
            .watch(path, crate::notify::RecursiveMode::Recursive)
            .unwrap();
    }

    crate::tokio::task::spawn_blocking(move || {
        for events in watch_rx {
            let e = events.unwrap().into_iter().map(|e| e.path).collect();
            tx.send(crate::Event::FileChanged(e)).unwrap();
        }
    });

    debouncer
}
