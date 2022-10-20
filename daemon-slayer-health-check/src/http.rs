use std::error::Error;

use daemon_slayer_core::health_check::HealthCheck;

#[cfg(feature = "http-health-check")]
#[derive(Clone)]
pub enum HttpRequestType {
    Get,
    Head,
}

#[cfg(feature = "http-health-check")]
#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
#[derive(Clone)]
pub struct HttpHealthCheck {
    request_type: HttpRequestType,
    url: reqwest::Url,
}

#[cfg(feature = "http-health-check")]
#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
impl HttpHealthCheck {
    pub fn new(
        request_type: HttpRequestType,
        url: impl reqwest::IntoUrl,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            request_type,
            url: url.into_url()?,
        })
    }
}

#[cfg(all(feature = "async-tokio", feature = "http-health-check"))]
#[async_trait::async_trait]
impl HealthCheck for HttpHealthCheckAsync {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let response = match &self.request_type {
            HttpRequestType::Get => reqwest::get(self.url.clone()).await?,
            HttpRequestType::Head => {
                reqwest::Client::builder()
                    .build()?
                    .head(self.url.clone())
                    .send()
                    .await?
            }
        };

        let status = response.status();
        if !status.is_success() {
            return Err(format!("Received status {status}"))?;
        }

        Ok(())
    }
}

#[cfg(all(feature = "blocking", feature = "http-health-check"))]
impl daemon_slayer_core::health_check::blocking::HealthCheck for HttpHealthCheckSync {
    fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let response = match &self.request_type {
            HttpRequestType::Get => reqwest::blocking::get(self.url.clone())?,
            HttpRequestType::Head => reqwest::blocking::Client::builder()
                .build()?
                .head(self.url.clone())
                .send()?,
        };

        let status = response.status();
        if !status.is_success() {
            return Err(format!("Received status {status}"))?;
        }
        Ok(())
    }
}
