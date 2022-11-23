use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
    time::Duration,
};

use futures::FutureExt;
use parity_tokio_ipc::{Connection, Endpoint};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    task::JoinHandle,
};

pub(crate) struct IpcClientStream {
    waker: Arc<Mutex<Option<Waker>>>,
    connection: Arc<Mutex<Option<Connection>>>,
}

impl IpcClientStream {
    pub(crate) fn new(addr: String) -> Self {
        let waker: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));
        let waker_ = waker.clone();
        let connection: Arc<Mutex<Option<Connection>>> = Arc::new(Mutex::new(None));
        let connection_ = connection.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(connection) = Endpoint::connect(&addr).await {
                    let waker = waker_.lock().unwrap().take();
                    let mut c = connection_.lock().unwrap();
                    *c = Some(connection);
                    if let Some(w) = waker {
                        w.wake();
                        return;
                    } else {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        Self { waker, connection }
    }
}

impl AsyncRead for IpcClientStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut connection = self.connection.lock().unwrap();
        match &mut *connection {
            Some(connection) => Pin::new(connection).poll_read(cx, buf),
            None => {
                let mut waker = self.waker.lock().unwrap();
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl AsyncWrite for IpcClientStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut connection = self.connection.lock().unwrap();
        match &mut *connection {
            Some(connection) => Pin::new(connection).poll_write(cx, buf),
            None => {
                let mut waker = self.waker.lock().unwrap();
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut connection = self.connection.lock().unwrap();
        match &mut *connection {
            Some(connection) => Pin::new(connection).poll_flush(cx),
            None => {
                let mut waker = self.waker.lock().unwrap();
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut connection = self.connection.lock().unwrap();
        match &mut *connection {
            Some(connection) => Pin::new(connection).poll_shutdown(cx),
            None => {
                let mut waker = self.waker.lock().unwrap();
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
