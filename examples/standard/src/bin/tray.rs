use std::{env::current_exe, path::PathBuf};

use daemon_slayer::{
    client::{self, config::Level},
    core::BoxedError,
    tray::Tray,
};

#[tokio::main]
pub async fn main() -> Result<(), BoxedError> {
    let icon_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png"));
    let manager = client::builder(
        standard::label(),
        current_exe()?
            .parent()
            .expect("Current exe should have a parent")
            .join("standard-server")
            .try_into()?,
    )
    .with_description("test service")
    .with_arg(&standard::run_argument())
    .with_service_level(if cfg!(windows) {
        Level::System
    } else {
        Level::User
    })
    .build()
    .await?;

    Tray::with_default_handler(manager, icon_path).start().await;
    Ok(())
}
