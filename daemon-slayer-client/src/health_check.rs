use std::error::Error;

use reqwest::{IntoUrl, Url};

#[cfg(feature = "async-tokio")]
#[async_trait::async_trait]
pub trait HealthCheckAsync {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
}

#[cfg(feature = "blocking")]
pub trait HealthCheckSync {
    fn invoke(&mut self) -> Result<(), Box<dyn Error>>;
}

#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
pub struct IpcHealthCheck;

pub enum RequestType {
    Get,
    Head,
}

#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
pub struct HttpHealthCheck {
    request_type: RequestType,
    url: Url,
}

#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
impl HttpHealthCheck {
    pub fn new(request_type: RequestType, url: impl IntoUrl) -> Self {
        Self {
            request_type,
            url: url.into_url().unwrap(),
        }
    }
}

#[cfg(feature = "async-tokio")]
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

#[cfg(feature = "blocking")]
impl HealthCheckSync for HttpHealthCheckSync {
    fn invoke(&mut self) -> Result<(), Box<dyn Error>> {
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
