#[cfg(not(feature = "async-tokio"))]
mod app;
#[cfg(feature = "async-tokio")]
mod async_app;

#[cfg(not(feature = "async-tokio"))]
pub fn main() {
    app::run_app();
}

#[cfg(feature = "async-tokio")]
#[tokio::main]
async fn main() {
    async_app::run_app().await;
}
