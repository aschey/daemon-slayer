use std::io;

use async_trait::async_trait;

#[async_trait]
pub trait AsyncNotification {
    type Output;

    async fn show(&self) -> io::Result<Self::Output>;
}

pub trait BlockingNotification {
    type Output;

    fn show_blocking(&self) -> io::Result<Self::Output>;
}
