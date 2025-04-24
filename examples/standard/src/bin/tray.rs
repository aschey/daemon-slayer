use std::env::current_exe;
use std::path::PathBuf;

use daemon_slayer::client::config::Level;
use daemon_slayer::client::{self};
use daemon_slayer::core::BoxedError;
use daemon_slayer::tray::Tray;

pub fn main() -> Result<(), BoxedError> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap();
    let _guard = rt.enter();

    let icon_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png"));
    let manager = rt
        .block_on(
            client::builder(
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
            .build(),
        )
        .unwrap();

    Tray::with_default_handler(manager, icon_path).run();
    Ok(())
}
