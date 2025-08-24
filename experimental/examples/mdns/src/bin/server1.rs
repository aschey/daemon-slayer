use std::collections::HashMap;
use std::future::IntoFuture;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::middleware::Next;
use axum::routing::get;
use axum::{Router, middleware};
use confique::Config;
use daemon_slayer::build_info::cli::BuildInfoCliProvider;
use daemon_slayer::build_info::vergen_pretty::{self, Style};
use daemon_slayer::cli::Cli;
use daemon_slayer::config::cli::ConfigCliProvider;
use daemon_slayer::config::server::ConfigService;
use daemon_slayer::config::{AppConfig, ConfigDir};
use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::error_handler::ErrorSink;
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::color_eyre::eyre;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::server::LoggingUpdateService;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{self, LoggerBuilder, ReloadHandle};
use daemon_slayer::server::cli::ServerCliProvider;
use daemon_slayer::server::futures::StreamExt;
use daemon_slayer::server::socket_activation::{SocketResult, get_activation_sockets};
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, ServiceContext, Signal, SignalHandler,
};
use daemon_slayer::signals::SignalListener;
use daemon_slayer_logging::time::format_description::well_known::Rfc3339;
use daemon_slayer_logging::tracing_subscriber::fmt::time::OffsetTime;
use daemon_slayer_network::cli::NetworkCliProvider;
use daemon_slayer_network::discovery::{
    DiscoveryBroadcastService, DiscoveryProtocol, DiscoveryQueryService,
};
use daemon_slayer_network::tokio_util::future::FutureExt;
use daemon_slayer_network::{BroadcastServiceName, QueryServiceName, ServiceProtocol};
use derive_more::AsRef;
use mdns::SOCKET_NAME;
use tokio::sync::mpsc::Sender;
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Debug, Config, AsRef, Default, Clone)]
struct MyConfig {
    #[as_ref]
    #[config(nested)]
    logging_config: logging::UserConfig,
}

#[derive(Clone)]
pub struct AppData {
    config: AppConfig<MyConfig>,
    reload_handle: ReloadHandle,
}

pub fn main() -> Result<(), ErrorSink> {
    async_main(OffsetTime::local_rfc_3339().unwrap())
}

#[tokio::main]
async fn async_main(offset_time: OffsetTime<Rfc3339>) -> Result<(), ErrorSink> {
    let guard = daemon_slayer::logging::init();
    let result = run(offset_time)
        .await
        .map_err(|e| ErrorSink::new(eyre::eyre!(e)));
    drop(guard);
    result
}

async fn run(offset_time: OffsetTime<Rfc3339>) -> Result<(), BoxedError> {
    let app_config =
        AppConfig::<MyConfig>::builder(ConfigDir::ProjectDir(mdns::label())).build()?;

    let logger_builder = LoggerBuilder::new(ServiceHandler::label(), offset_time);
    let pretty = vergen_pretty::Pretty::builder()
        .env(vergen_pretty::vergen_pretty_env!())
        .category(false)
        .key_style(Style::new().bold().cyan())
        .value_style(Style::new())
        .build();
    let mut cli = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::new(
            &mdns::run_argument(),
        ))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(ConfigCliProvider::new(app_config.clone()))
        .with_provider(BuildInfoCliProvider::new(pretty))
        .with_provider(NetworkCliProvider::default())
        .initialize()?;

    let (logger, reload_handle) = cli
        .take_provider::<LoggingCliProvider<Rfc3339>>()
        .get_logger_with_reload(app_config.clone())?;

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
    signal_store: BroadcastEventStore<Signal>,
    context: ServiceContext,
}

impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = AppData;

    fn label() -> Label {
        mdns::label()
    }

    async fn new(
        context: ServiceContext,
        input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let input_data = input_data.unwrap();
        let signal_listener = SignalListener::termination();
        let signal_store = signal_listener.get_event_store();
        context.spawn(signal_listener);

        let config_service = ConfigService::new(input_data.config);
        let file_events = config_service.get_event_store();
        context.spawn(config_service);
        context.spawn(LoggingUpdateService::new(
            input_data.reload_handle,
            file_events,
        ));

        context.spawn(DiscoveryBroadcastService::new(
            DiscoveryProtocol::Both { udp_port: 33533 },
            BroadcastServiceName::new("test", "discovery-test"),
            ServiceProtocol::Tcp,
            9000,
            HashMap::from_iter([("test".to_owned(), "true".to_owned())]),
        ));

        let discovery_query_service = DiscoveryQueryService::new(
            DiscoveryProtocol::Both { udp_port: 33534 },
            QueryServiceName::new("discovery-test2"),
            ServiceProtocol::Tcp,
        );

        let discovery_query_store = discovery_query_service.get_event_store();

        context.spawn(discovery_query_service);

        context.spawn((
            "discovery_checker",
            move |context: ServiceContext| async move {
                let mut discovery_events = discovery_query_store.subscribe_events();
                tokio::time::sleep(Duration::from_secs(3)).await;
                while let Some(Ok(event)) = discovery_events
                    .next()
                    .with_cancellation_token(context.cancellation_token())
                    .await
                    .flatten()
                {
                    info!("Discovery event {event:?}");
                    for addr in event.ip_addresses {
                        let response = reqwest::Client::new()
                            .get(format!("http://{addr}:{}/health", event.port))
                            .send()
                            .await;
                        info!("HTTP response {response:?}");
                    }
                }
                Ok(())
            },
        ));

        Ok(Self {
            signal_store,
            context,
        })
    }

    async fn run_service<F: FnOnce() + Send>(self, notify_ready: F) -> Result<(), Self::Error> {
        info!("running service");
        notify_ready();

        let mut socket_result = get_activation_sockets(mdns::sockets(9000)).await?;
        let is_activated = socket_result.is_activated;

        let socket = socket_result
            .sockets
            .remove(SOCKET_NAME)
            .ok_or("missing socket")?
            .remove(0);

        let SocketResult::Tcp(listener) = socket else {
            return Err("invalid socket config")?;
        };

        let (refresh_tx, mut refresh_rx) = tokio::sync::mpsc::channel(32);

        let mut app = Router::new()
            .route("/hello/:name", get(greeter))
            .route("/health", get(health))
            .layer(TraceLayer::new_for_http());
        if is_activated {
            app = app.layer(middleware::from_fn_with_state(
                refresh_tx,
                |State(tx): State<Sender<()>>, request, next: Next| async move {
                    tx.try_send(()).unwrap();
                    next.run(request).await
                },
            ));
        }

        let mut signals = self.signal_store.subscribe_events();

        if let Some(res) = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                if is_activated {
                    loop {
                        let timeout = tokio::time::sleep(Duration::from_secs(10));
                        tokio::select! {
                            _ = signals.next() => {
                                info!("Got shutdown signal");
                            },
                            _ = timeout => {
                                info!("Terminating due to timeout");
                            }
                            res = refresh_rx.recv() => {
                                if res.is_some() {
                                    continue;
                                }
                            }
                        }
                        return;
                    }
                }
                signals.next().await;
                info!("Got shutdown signal");
            })
            .into_future()
            .with_cancellation_token(self.context.cancellation_token())
            .await
        {
            res?;
        }

        info!("Server terminated");

        Ok(())
    }
}

async fn greeter(Path(name): Path<String>) -> String {
    format!("Hello {name}")
}

async fn health() -> &'static str {
    "healthy"
}
