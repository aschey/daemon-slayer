use super::ipc_command::IpcCommand;
use once_cell::sync::OnceCell;
use parity_tokio_ipc::Endpoint;
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Once,
    },
    time::Duration,
};
use tokio::io::AsyncWriteExt;
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
                        client
                            .write_all(&buf)
                            .await
                            .expect("Unable to write message to client");
                    }
                    IpcCommand::Flush => {
                        client.flush().await.unwrap();
                        return;
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
