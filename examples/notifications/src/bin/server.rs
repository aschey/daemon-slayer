use confique::Config;
use daemon_slayer::{
    cli::Cli,
    config::{cli::ConfigCliProvider, server::ConfigService, AppConfig, ConfigDir},
    core::{notify::AsyncNotification, BoxedError, CancellationToken, Label},
    error_handler::{cli::ErrorHandlerCliProvider, color_eyre::eyre, ErrorSink},
    logging::{
        self, cli::LoggingCliProvider, server::LoggingUpdateService,
        tracing_subscriber::util::SubscriberInitExt, LoggerBuilder, ReloadHandle,
    },
    notify::{
        dialog::{cli::DialogCliProvider, Alert, Confirm, MessageDialog},
        notification::{cli::NotifyCliProvider, Notification},
        NotificationService,
    },
    server::{cli::ServerCliProvider, Handler, ServiceContext, Signal, SignalHandler},
    signals::SignalListener,
};
use derive_more::AsRef;
use std::time::{Duration, Instant};
use tracing::{error, info};

#[derive(Debug, Config, AsRef, Default, Clone)]
struct MyConfig {
    #[as_ref]
    #[config(nested)]
    logging_config: logging::UserConfig,
}

#[tokio::main]
pub async fn main() -> Result<(), ErrorSink> {
    let guard = daemon_slayer::logging::init();
    let result = run().await.map_err(|e| ErrorSink::new(eyre::eyre!(e)));
    drop(guard);
    result
}

#[derive(Clone)]
pub struct AppData {
    config: AppConfig<MyConfig>,
    reload_handle: ReloadHandle,
}

async fn run() -> Result<(), BoxedError> {
    let app_config =
        AppConfig::<MyConfig>::builder(ConfigDir::ProjectDir(notifications::label())).build()?;

    let logger_builder =
        LoggerBuilder::new(ServiceHandler::label()).with_config(app_config.clone());

    let mut cli = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::new(
            &notifications::run_argument(),
        ))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::default().with_notification(
            MessageDialog::<Alert>::new(ServiceHandler::label()).with_text("An error occurred"),
        ))
        .with_provider(ConfigCliProvider::new(app_config.clone()))
        .with_provider(NotifyCliProvider::new(ServiceHandler::label()))
        .with_provider(DialogCliProvider::new(ServiceHandler::label()))
        .initialize()?;

    let (logger, reload_handle) = cli.take_provider::<LoggingCliProvider>().get_logger()?;

    logger.init();

    cli.get_provider::<ServerCliProvider<ServiceHandler>>()
        .set_input_data(AppData {
            config: app_config,
            reload_handle: reload_handle.clone(),
        });

    cli.handle_input().await?;
    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    cancellation_token: CancellationToken,
}

#[daemon_slayer::core::async_trait]
impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = AppData;

    fn label() -> Label {
        notifications::label()
    }

    async fn new(
        mut context: ServiceContext,
        input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let input_data = input_data.unwrap();
        let signal_listener = SignalListener::all();
        let signal_store = signal_listener.get_event_store();

        context.add_service(signal_listener);
        context.add_service(
            NotificationService::new(signal_store, |signal| {
                if let Ok(signal) = signal {
                    if signal != Signal::SIGCHLD {
                        return Some(Notification::new(Self::label()).summary("Signal received"));
                    } else {
                        return None;
                    }
                }

                Some(Notification::new(Self::label()).summary("Signal received"))
            })
            .with_shutdown_timeout(Duration::from_millis(100)),
        );

        let config_service = ConfigService::new(input_data.config);
        let file_events = config_service.get_event_store();
        context.add_service(config_service);
        context.add_service(LoggingUpdateService::new(
            input_data.reload_handle,
            file_events,
        ));

        Ok(Self {
            cancellation_token: context.cancellation_token(),
        })
    }

    async fn run_service<F: FnOnce() + Send>(mut self, notify_ready: F) -> Result<(), Self::Error> {
        info!("running service");
        notify_ready();
        let run_service = MessageDialog::<Confirm>::new(Self::label())
            .with_text("Run the service?")
            .show()
            .await?;
        if !run_service {
            return Ok(());
        }

        let start_time = Instant::now();
        loop {
            match tokio::time::timeout(Duration::from_secs(1), self.cancellation_token.cancelled())
                .await
            {
                Ok(_) => {
                    info!("stopping service");
                    return Err("Simulated error".into());
                }
                Err(_) => {
                    if let Err(e) = Notification::new(Self::label())
                        .summary(format!(
                            "Run time: {} seconds",
                            Instant::now().duration_since(start_time).as_secs()
                        ))
                        .show()
                        .await
                    {
                        error!("Error showing notification: {e}");
                    }
                }
            }
        }
    }
}
