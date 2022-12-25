use std::env::current_exe;

use clap::Parser;
use daemon_slayer::{
    client::{
        self,
        config::{
            windows::{ServiceAccess, Trustee, WindowsConfig},
            Level,
        },
    },
    core::BoxedError,
};

#[derive(clap::Parser, Debug)]
enum Arg {
    /// Install the service
    Install,
    /// Uninstall the service
    Uninstall,
    /// Retrieve information about the service status
    Info,
    /// Start the service
    Start,
    /// Stop the service
    Stop,
    /// Restart the service
    Restart,
}

#[derive(clap::Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    arg: Arg,
}

#[tokio::main]
pub async fn main() -> Result<(), BoxedError> {
    let manager = client::builder(
        minimal_separate::label(),
        current_exe()?
            .parent()
            .expect("Current exe should have a parent")
            .join("minimal-server")
            .try_into()?,
    )
    .with_description("test service")
    .with_args(["run"])
    .with_service_level(if cfg!(windows) {
        Level::System
    } else {
        Level::User
    })
    .with_windows_config(WindowsConfig::default().with_additional_access(
        Trustee::CurrentUser,
        ServiceAccess::Start | ServiceAccess::Stop | ServiceAccess::ChangeConfig,
    ))
    .build()?;

    match Cli::parse().arg {
        Arg::Install => {
            manager.install()?;
        }
        Arg::Uninstall => {
            manager.uninstall()?;
        }
        Arg::Info => {
            println!("{}", manager.info()?.pretty_print());
        }
        Arg::Start => {
            manager.start()?;
        }
        Arg::Stop => {
            manager.stop()?;
        }
        Arg::Restart => {
            manager.restart()?;
        }
    }

    Ok(())
}
