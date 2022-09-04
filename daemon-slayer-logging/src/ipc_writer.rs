use super::ipc_command::IpcCommand;
use futures::StreamExt;
use once_cell::sync::OnceCell;
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Once,
    },
    time::Duration,
};
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tracing_subscriber::fmt::MakeWriter;

static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);
static SENDER: OnceCell<tokio::sync::mpsc::Sender<IpcCommand>> = OnceCell::new();
static HANDLE: OnceCell<tokio::sync::Mutex<tokio::task::JoinHandle<()>>> = OnceCell::new();

pub(crate) struct IpcWriter;

pub(crate) struct WorkerGuard;

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        futures::executor::block_on(async {
            SENDER
                .get()
                .unwrap()
                .send_timeout(IpcCommand::Flush, Duration::from_millis(100))
                .await
                .unwrap();
        });
    }
}

impl IpcWriter {
    pub(crate) fn new() -> (Self, WorkerGuard) {
        (Self, WorkerGuard)
    }

    fn init(&self, mut rx: tokio::sync::mpsc::Receiver<IpcCommand>) {
        let handle = tokio::spawn(async move {
            loop {
                let mut last_connect = tokio::time::Instant::now();
                let mut client = match Endpoint::connect("/tmp/daemon_slayer.sock").await {
                    Ok(client) => client,
                    Err(_) => loop {
                        tokio::select! {
                            client = Endpoint::connect("/tmp/daemon_slayer.sock"), if tokio::time::Instant::now().duration_since(last_connect) > tokio::time::Duration::from_secs(1) => {
                                match client {
                                    Ok(client) => break client,
                                    Err(_) => {
                                        last_connect = tokio::time::Instant::now();
                                    }
                                }
                            },
                            cmd = rx.recv() => {
                                match cmd {
                                    Some(IpcCommand::Write(_)) => {},
                                    _ => {
                                        return;
                                    },

                                }
                            },
                            _ =  tokio::time::sleep(Duration::from_millis(1000)) => {
                            },
                        }
                    },
                };

                while let Some(cmd) = rx.recv().await {
                    match cmd {
                        IpcCommand::Write(buf) => {
                            if client.write_all(&buf).await.is_err() {
                                break;
                            };
                        }
                        IpcCommand::Flush => {
                            let _ = client.flush().await;
                            return;
                        }
                    }
                }
            }
        });
        HANDLE.set(tokio::sync::Mutex::new(handle)).unwrap();
    }
}

impl MakeWriter<'_> for IpcWriter {
    type Writer = IpcWriter;

    fn make_writer(&'_ self) -> Self::Writer {
        if !IS_INITIALIZED.swap(true, Ordering::SeqCst) {
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            SENDER.get_or_init(|| tx);

            self.init(rx);
        }

        Self
    }
}

impl std::io::Write for IpcWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let b = buf.to_owned();

        tokio::spawn(async move {
            if let Err(e) = SENDER.get().unwrap().send(IpcCommand::Write(b)).await {
                println!("IpcWriterInstance Err writing {e}");
            }
        });

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        tokio::spawn(async move {
            if let Err(e) = SENDER.get().unwrap().send(IpcCommand::Flush).await {
                println!("IpcWriterInstance Err flushing {e:?}");
            }
        });

        Ok(())
    }
}

pub async fn run_ipc_server(tx: tokio::sync::mpsc::Sender<String>) {
    let mut endpoint = Endpoint::new("/tmp/daemon_slayer.sock".to_owned());
    endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

    let mut incoming = endpoint.incoming().expect("failed to open new socket");
    let mut buf = [0; 2048];
    while let Some(result) = incoming.next().await {
        match result {
            Ok(stream) => {
                let (mut reader, _) = split(stream);

                loop {
                    let bytes = match reader.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(bytes) => bytes,
                        Err(_) => break,
                    };

                    if tx
                        .send(std::str::from_utf8(&buf[0..bytes]).unwrap().to_string())
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
            _ => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
}
