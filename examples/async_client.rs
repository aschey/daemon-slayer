use std::env::args;
use std::time::{Duration, Instant};

use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{Action, CliHandler, Command};

use daemon_slayer_cli::ClientCli;
use futures::{SinkExt, StreamExt};

pub fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        //.with_service_level(ServiceLevel::User);
        let manager = ServiceManager::builder("daemon_slayer_test_service")
            .with_description("test service")
            .with_args(["run"])
            .build()
            .unwrap();

        let cli = ClientCli::new(manager);
        cli.handle_input().await.unwrap();
    });
}
