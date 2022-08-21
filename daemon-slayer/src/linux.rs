use std::env::current_exe;

use systemd_client::{
    create_unit_configuration_file, delete_unit_configuration_file, manager, unit,
    ServiceConfiguration, ServiceUnitConfiguration, UnitActiveStateType, UnitConfiguration,
    UnitLoadStateType, UnitSubStateType,
};

use crate::{service_manager::ServiceManager, service_state::ServiceState};

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

                $service_main_func(vec![], sender, receiver)
            }

            pub fn $service_func_name()  {
                [<$service_func_name _main>]();
            }
        }
    };
}

pub struct Manager {
    service_name: String,
}
impl ServiceManager for Manager {
    fn new<T: Into<String>>(service_name: T) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    fn install<'a, T: Into<&'a str>>(&self, args: impl IntoIterator<Item = T>) {
        let unit_config = UnitConfiguration::builder().description("test service");

        let path_str = current_exe().unwrap().to_string_lossy().to_string();
        let service_args = std::iter::once(&path_str[..])
            .chain(args.into_iter().map(|a| a.into()))
            .collect();
        let service_config = ServiceConfiguration::builder().exec_start(service_args);
        let svc_unit = ServiceUnitConfiguration::builder()
            .unit(unit_config)
            .service(service_config)
            .build();
        let svc_unit_literal = format!("{}", svc_unit);
        create_unit_configuration_file(
            &format!("{}.service", self.service_name),
            svc_unit_literal.as_bytes(),
        )
        .unwrap();
    }

    fn uninstall(&self) {
        let _ = delete_unit_configuration_file(&format!("{}.service", self.service_name));
    }

    fn start(&self) {
        let client = manager::build_blocking_proxy().unwrap();
        client
            .start_unit(&format!("{}.service", self.service_name), "replace")
            .unwrap();
    }

    fn stop(&self) {
        if self.query_status() == ServiceState::Started {
            let client = manager::build_blocking_proxy().unwrap();
            client
                .stop_unit(&format!("{}.service", self.service_name), "replace")
                .unwrap();
        }
    }

    fn query_status(&self) -> ServiceState {
        let client = manager::build_blocking_proxy().unwrap();
        client.reload().unwrap();
        client.reset_failed().unwrap();

        let svc_unit_path = client
            .load_unit(&format!("{}.service", self.service_name))
            .unwrap();

        let client = unit::build_blocking_proxy(svc_unit_path).unwrap();
        let props = client.get_properties().unwrap();
        match (props.load_state, props.active_state, props.sub_state) {
            (UnitLoadStateType::Loaded, UnitActiveStateType::Active, UnitSubStateType::Running) => {
                ServiceState::Started
            }
            (UnitLoadStateType::NotFound, _, _) => ServiceState::NotInstalled,
            _ => ServiceState::Stopped,
        }
    }
}
