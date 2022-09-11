use std::error::Error;

#[cfg(feature = "http-health-check")]
pub enum HttpRequestType {
    Get,
    Head,
}

#[cfg(feature = "http-health-check")]
#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
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
impl super::HealthCheckAsync for HttpHealthCheckAsync {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        match &self.request_type {
            HttpRequestType::Get => {
                reqwest::get(self.url.clone()).await?;
            }
            HttpRequestType::Head => {
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
impl super::HealthCheckSync for HttpHealthCheckSync {
    fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        match &self.request_type {
            HttpRequestType::Get => {
                reqwest::blocking::get(self.url.clone())?;
            }
            HttpRequestType::Head => {
                reqwest::blocking::Client::builder()
                    .build()?
                    .head(self.url.clone())
                    .send()?;
            }
        };
        Ok(())
    }
}
