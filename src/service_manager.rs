#[cfg(feature = "async-tokio")]
use async_trait::async_trait;
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
    fn install(&self) -> Result<()>;
    fn uninstall(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn query_status(&self) -> Result<ServiceStatus>;
}

#[cfg(feature = "async-tokio")]
pub type StopHandler = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[cfg(not(feature = "async-tokio"))]
pub type StopHandler = Box<dyn Fn() + Send>;

#[cfg(feature = "async-tokio")]
#[async_trait]
pub trait ServiceHandler {
    fn new() -> Self;
    fn get_service_name<'a>() -> &'a str;
    fn get_stop_handler(&mut self) -> StopHandler;
    async fn run_service(mut self) -> u32;
}

#[cfg(not(feature = "async-tokio"))]
pub trait ServiceHandler {
    fn new() -> Self;
    fn get_service_name<'a>() -> &'a str;
    fn get_stop_handler(&mut self) -> StopHandler;
    fn run_service(mut self) -> u32;
}
