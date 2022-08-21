use std::{ffi::OsString, env::current_exe};

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
       
    }

    pub fn uninstall(&self) {
      
    }

    pub fn start(&self) {
        
    }

    pub fn stop(&self) {
        
    }

    pub fn query_status(&self) -> ServiceState {
       ServiceState::NotInstalled
    }

    pub fn is_installed(&self) -> bool {
        self.query_status() != ServiceState::NotInstalled
    }
}