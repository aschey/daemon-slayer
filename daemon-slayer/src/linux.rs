use std::{env::current_exe, ffi::OsString};

use systemd_client::{
    create_unit_configuration_file, delete_unit_configuration_file, manager, unit,
    ServiceConfiguration, ServiceUnitConfiguration, UnitActiveStateType, UnitConfiguration,
    UnitLoadStateType, UnitSubStateType,
};

use crate::service_state::ServiceState;

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
    service_name: OsString,
}
impl Manager {
    pub fn new<T: Into<OsString>>(service_name: T) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    pub fn install<T: Into<OsString>>(&self, args: Vec<T>) {
        let mut service_binary_path = vec![current_exe().unwrap().to_string_lossy().to_string()];
        let unit_config = UnitConfiguration::builder().description("test service");
        let service_args_: Vec<OsString> = args.into_iter().map(|a| a.into()).collect();
        let mut service_args: Vec<String> = service_args_
            .into_iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        service_binary_path.append(&mut service_args);
        let service_config = ServiceConfiguration::builder()
            .exec_start(service_binary_path.iter().map(AsRef::as_ref).collect());
        let svc_unit = ServiceUnitConfiguration::builder()
            .unit(unit_config)
            .service(service_config)
            .build();
        let svc_unit_literal = format!("{}", svc_unit);
        create_unit_configuration_file(
            &format!("{}.service", self.service_name.to_string_lossy().to_owned()),
            svc_unit_literal.as_bytes(),
        )
        .unwrap();
    }

    pub fn uninstall(&self) {
        let _ = delete_unit_configuration_file(&format!(
            "{}.service",
            self.service_name.to_string_lossy().to_owned()
        ));
    }

    pub fn start(&self) {
        let client = manager::build_blocking_proxy().unwrap();
        client
            .start_unit(
                &format!("{}.service", self.service_name.to_string_lossy().to_owned()),
                "replace",
            )
            .unwrap();
    }

    pub fn stop(&self) {
        let client = manager::build_blocking_proxy().unwrap();
        client
            .stop_unit(
                &format!("{}.service", self.service_name.to_string_lossy().to_owned()),
                "replace",
            )
            .unwrap();
    }

    pub fn query_status(&self) -> ServiceState {
        let client = manager::build_blocking_proxy().unwrap();
        client.reload().unwrap();
        client.reset_failed().unwrap();

        let svc_unit_path = client
            .load_unit(&format!(
                "{}.service",
                self.service_name.to_string_lossy().to_owned()
            ))
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

    pub fn is_installed(&self) -> bool {
        self.query_status() != ServiceState::NotInstalled
    }
}
