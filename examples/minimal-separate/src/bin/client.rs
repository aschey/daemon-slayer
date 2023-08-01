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
    Status,
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
    .with_arg(&minimal_separate::run_argument())
    .with_service_level(if cfg!(windows) {
        Level::System
    } else {
        Level::User
    })
    .with_windows_config(WindowsConfig::default().with_additional_access(
        Trustee::CurrentUser,
        ServiceAccess::Start | ServiceAccess::Stop | ServiceAccess::ChangeConfig,
    ))
    .build()
    .await?;

    match Cli::parse().arg {
        Arg::Install => {
            manager.install().await?;
        }
        Arg::Uninstall => {
            manager.uninstall().await?;
        }
        Arg::Status => {
            println!("{}", manager.status().await?.pretty_print());
        }
        Arg::Start => {
            manager.start().await?;
        }
        Arg::Stop => {
            manager.stop().await?;
        }
        Arg::Restart => {
            manager.restart().await?;
        }
    }

    Ok(())
}
