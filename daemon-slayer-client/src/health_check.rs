use std::error::Error;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[maybe_async_cfg::maybe(
    idents(Service, HealthCheck),
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait HealthCheck {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
}

#[cfg(feature = "ipc-health-check")]
#[cfg(feature = "async-tokio")]
pub struct IpcHealthCheckAsync {
    sock_path: String,
    read_buf: [u8; 256],
}

#[cfg(feature = "ipc-health-check")]
impl IpcHealthCheckAsync {
    pub fn new(sock_path: impl Into<String>) -> Self {
        Self {
            sock_path: sock_path.into(),
            read_buf: [0u8; 256],
        }
    }
}

#[cfg(feature = "ipc-health-check")]
#[async_trait::async_trait]
impl HealthCheckAsync for IpcHealthCheckAsync {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut client = parity_tokio_ipc::Endpoint::connect(&self.sock_path).await?;
        let _ = client.write_u8(0).await?;

        let _ = client.read(&mut self.read_buf).await?;
        Ok(())
    }
}

#[cfg(feature = "http-health-check")]
pub enum RequestType {
    Get,
    Head,
}

#[cfg(feature = "http-health-check")]
#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
pub struct HttpHealthCheck {
    request_type: RequestType,
    url: reqwest::Url,
}

#[cfg(feature = "http-health-check")]
#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
impl HttpHealthCheck {
    pub fn new(request_type: RequestType, url: impl reqwest::IntoUrl) -> Self {
        Self {
            request_type,
            url: url.into_url().unwrap(),
        }
    }
}

#[cfg(all(feature = "async-tokio", feature = "http-health-check"))]
#[async_trait::async_trait]
impl HealthCheckAsync for HttpHealthCheckAsync {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        match &self.request_type {
            RequestType::Get => {
                reqwest::get(self.url.clone()).await?;
            }
            RequestType::Head => {
                reqwest::Client::builder()
                    .build()?
                    .head(self.url.clone())
                    .send()
                    .await?;
            }
        };
        Ok(())
    }
}

#[cfg(all(feature = "blocking", feature = "http-health-check"))]
impl HealthCheckSync for HttpHealthCheckSync {
    fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        match &self.request_type {
            RequestType::Get => {
                reqwest::blocking::get(self.url.clone())?;
            }
            RequestType::Head => {
                reqwest::blocking::Client::builder()
                    .build()?
                    .head(self.url.clone())
                    .send()?;
            }
        };
        Ok(())
    }
}

#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
pub struct TcpHealthCheck;

#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
pub struct GrpcHealthCheck;
