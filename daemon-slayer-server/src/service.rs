#[maybe_async::maybe_async]
pub trait Service {
    async fn run_service_main() -> u32;
    #[cfg(feature = "direct")]
    async fn run_service_direct(self) -> u32;
}
