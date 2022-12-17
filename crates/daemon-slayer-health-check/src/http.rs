use std::error::Error;

use daemon_slayer_core::health_check::HealthCheck;

#[derive(Clone)]
pub enum HttpRequestType {
    Get,
    Head,
}

#[derive(Clone)]
pub struct HttpHealthCheck {
    request_type: HttpRequestType,
    url: reqwest::Url,
}

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

#[async_trait::async_trait]
impl HealthCheck for HttpHealthCheck {
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
