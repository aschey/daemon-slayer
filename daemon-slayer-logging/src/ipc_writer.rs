use super::ipc_command::IpcCommand;
use once_cell::sync::OnceCell;
use parity_tokio_ipc::Endpoint;
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use tracing_subscriber::fmt::MakeWriter;

static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);
static SENDER: OnceCell<tokio::sync::mpsc::Sender<IpcCommand>> = OnceCell::new();

pub(crate) struct IpcWriter;

pub(crate) struct WorkerGuard;

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        tokio::spawn(async {
            SENDER.get().unwrap().send(IpcCommand::Flush).await.unwrap();
        });

        while IS_INITIALIZED.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl IpcWriter {
    pub(crate) fn new() -> (Self, WorkerGuard) {
        (Self, WorkerGuard)
    }

    fn init(&self, mut rx: tokio::sync::mpsc::Receiver<IpcCommand>) {
        tokio::spawn(async move {
            let mut client = loop {
                tokio::select! {
                    client = Endpoint::connect("/tmp/daemon_slayer.sock") => {
                        match client {
                            Ok(client) => break client,
                            Err(e) => {
                                println!("Error connecting {e:?}");
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                        }
                    },
                    cmd = rx.recv() => {
                        match cmd {
                            Some(IpcCommand::Write(_)) => {},
                            Some(IpcCommand::Flush) => {
                                IS_INITIALIZED.swap(false, Ordering::SeqCst);
                                return;
                            },
                            None => {
                                IS_INITIALIZED.swap(false, Ordering::SeqCst);
                                return;
                            }
                        }
                    }
                }
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
                        IS_INITIALIZED.swap(false, Ordering::SeqCst);
                        return;
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        });
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
        // if !IS_INITIALIZED.load(Ordering::SeqCst) {
        //     return Ok(buf.len());
        // }
        let b = buf.to_owned();

        tokio::spawn(async move {
            if let Err(e) = SENDER.get().unwrap().send(IpcCommand::Write(b)).await {
                println!("IpcWriterInstance Err writing {e}");
            }
        });

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // if !IS_INITIALIZED.load(Ordering::SeqCst) {
        //     return Ok(());
        // }

        tokio::spawn(async move {
            if let Err(e) = SENDER.get().unwrap().send(IpcCommand::Flush).await {
                println!("IpcWriterInstance Err flushing {e:?}");
            }
        });

        Ok(())
    }
}
