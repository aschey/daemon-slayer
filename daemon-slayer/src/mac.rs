use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use launchd::Launchd;

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
    fn run_launchctl(&self, args: Vec<&str>) -> String {
        let output = Command::new("launchctl")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(args)
            .output()
            .unwrap();

        if output.status.success() {
            String::from_utf8(output.stdout).unwrap()
        } else {
            String::from_utf8(output.stderr).unwrap()
        }
    }

    fn get_plist_path(&self) -> PathBuf {
        PathBuf::from("/Library/LaunchDaemons").join(format!("{}.plist", self.config.name))
    }
}

impl ServiceManager for Manager {
    fn new(config: ServiceConfig) -> Self {
        Self { config }
    }

    fn install(&self) {
        let file = Launchd::new(&self.config.name, &self.config.program)
            .unwrap()
            .with_program_arguments(self.config.args_iter().map(|a| a.to_owned()).collect());

        let path = self.get_plist_path();
        file.to_writer_xml(std::fs::File::create(&path).unwrap())
            .unwrap();

        self.run_launchctl(vec!["load", &path.to_string_lossy()]);
    }

    fn uninstall(&self) {
        let path = self.get_plist_path();
        self.run_launchctl(vec!["unload", &path.to_string_lossy()]);
        fs::remove_file(path).unwrap();
    }

    fn start(&self) {
        self.run_launchctl(vec!["start", &self.config.name]);
    }

    fn stop(&self) {
        self.run_launchctl(vec!["stop", &self.config.name]);
    }

    fn query_status(&self) -> ServiceStatus {
        let output = self.run_launchctl(vec!["print", &format!("system/{}", self.config.name)]);
        if output.starts_with("Could not find service") {
            return ServiceStatus::NotInstalled;
        }
        let s = output
            .split('\n')
            .into_iter()
            .filter(|line| line.trim().starts_with("state"))
            .map(|line| {
                line.split('=')
                    .collect::<Vec<_>>()
                    .get(1)
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();

        if s[0].trim() == "running" {
            ServiceStatus::Started
        } else {
            ServiceStatus::Stopped
        }
    }
}
