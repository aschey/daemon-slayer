#[cfg(feature = "async-tokio")]
use futures::Future;
#[cfg(feature = "async-tokio")]
use std::pin::Pin;
use std::{error::Error, result};

use crate::{service_builder::ServiceBuilder, service_status::ServiceStatus};

pub type Result<T> = result::Result<T, Box<dyn Error>>;
pub trait ServiceManager {
    fn builder(name: impl Into<String>) -> ServiceBuilder;
    fn new(name: impl Into<String>) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn from_builder(builder: ServiceBuilder) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn display_name(&self) -> &str;
    fn description(&self) -> &str;
    fn args(&self) -> &Vec<String>;
    fn install(&self) -> Result<()>;
    fn uninstall(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn query_status(&self) -> Result<ServiceStatus>;
}

#[maybe_async::async_impl]
pub type StopHandler = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[maybe_async::sync_impl]
pub type StopHandler = Box<dyn Fn() + Send>;

#[maybe_async::maybe_async]
pub trait ServiceHandler {
    fn new() -> Self;
    fn get_service_name<'a>() -> &'a str;
    fn get_stop_handler(&mut self) -> StopHandler;
    async fn run_service<F: FnOnce() + Send>(self, on_started: F) -> u32;
}

#[maybe_async::maybe_async]
pub trait Service {
    async fn run_service_main() -> u32;
    #[cfg(feature = "direct")]
    async fn run_service_direct(self) -> u32;
}
