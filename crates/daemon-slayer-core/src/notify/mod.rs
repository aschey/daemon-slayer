use std::io;

use async_trait::async_trait;

#[async_trait]
pub trait ShowNotification {
    type Output;

    async fn show(&self) -> io::Result<Self::Output>;
}
