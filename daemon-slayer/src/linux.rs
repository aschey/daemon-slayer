use systemd_client::{
    create_unit_configuration_file, delete_unit_configuration_file, manager, unit,
    ServiceConfiguration, ServiceUnitConfiguration, UnitActiveStateType, UnitConfiguration,
    UnitLoadStateType, UnitSubStateType,
};

use crate::{
    service_config::ServiceConfig, service_manager::ServiceManager, service_status::ServiceStatus,
};

#[macro_export]
macro_rules! define_service {
    ($service_name:ident, $service_func_name:ident, $define_handler:expr, $on_stop:expr, $service_main_func:ident) => {
        $crate::paste::paste! {
            pub fn [<$service_func_name _main>]() -> u32 {
                let (sender, receiver) = $define_handler;
                let sender_ = sender.clone();
                $crate::tokio::spawn(async move {
                    use $crate::futures::stream::StreamExt;
                    let signals = $crate::signal_hook_tokio::Signals::new(&[
                        $crate::signal_hook::consts::signal::SIGHUP,
                        $crate::signal_hook::consts::signal::SIGTERM,
                        $crate::signal_hook::consts::signal::SIGINT,
                        $crate::signal_hook::consts::signal::SIGQUIT,
                    ])
                    .unwrap();
                    //let handle = signals.handle();

                    let mut signals = signals.fuse();
                    while let Some(signal) = signals.next().await {
                        match signal {
                            $crate::signal_hook::consts::signal::SIGTERM
                            | $crate::signal_hook::consts::signal::SIGINT
                            | $crate::signal_hook::consts::signal::SIGQUIT
                            | $crate::signal_hook::consts::signal::SIGHUP => {
                                $on_stop(&sender_);
                            }
                            _ => {}
                        }
                    }
                });

                $service_main_func(sender, receiver)
            }

            pub fn $service_func_name()  {
                [<$service_func_name _main>]();
            }
        }
    };
}

pub struct Manager {
    config: ServiceConfig,
}

impl Manager {
    fn service_file_name(&self) -> String {
        format!("{}.service", self.config.name)
    }
}
impl ServiceManager for Manager {
    fn new(config: ServiceConfig) -> Self {
        Self { config }
    }

    fn install(&self) {
        let unit_config = UnitConfiguration::builder().description(&self.config.description);

        let service_config = ServiceConfiguration::builder()
            .exec_start(self.config.full_args_iter().map(|a| &a[..]).collect());
        let svc_unit = ServiceUnitConfiguration::builder()
            .unit(unit_config)
            .service(service_config)
            .build();
        let svc_unit_literal = format!("{}", svc_unit);
        create_unit_configuration_file(&self.service_file_name(), svc_unit_literal.as_bytes())
            .unwrap();
    }

    fn uninstall(&self) {
        let _ = delete_unit_configuration_file(&self.service_file_name());
    }

    fn start(&self) {
        let client = manager::build_blocking_proxy().unwrap();
        client
            .start_unit(&self.service_file_name(), "replace")
            .unwrap();
    }

    fn stop(&self) {
        if self.query_status() == ServiceStatus::Started {
            let client = manager::build_blocking_proxy().unwrap();
            client
                .stop_unit(&self.service_file_name(), "replace")
                .unwrap();
        }
    }

    fn query_status(&self) -> ServiceStatus {
        let client = manager::build_blocking_proxy().unwrap();
        client.reload().unwrap();
        client.reset_failed().unwrap();

        let svc_unit_path = client.load_unit(&self.service_file_name()).unwrap();

        let client = unit::build_blocking_proxy(svc_unit_path).unwrap();
        let props = client.get_properties().unwrap();
        match (props.load_state, props.active_state, props.sub_state) {
            (UnitLoadStateType::Loaded, UnitActiveStateType::Active, UnitSubStateType::Running) => {
                ServiceStatus::Started
            }
            (UnitLoadStateType::NotFound, _, _) => ServiceStatus::NotInstalled,
            _ => ServiceStatus::Stopped,
        }
    }
}
