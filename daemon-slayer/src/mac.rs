use std::{
    env::current_exe,
    fs, iter,
    path::PathBuf,
    process::{Command, Stdio},
};

use launchd::Launchd;

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
    service_name: String,
}

impl Manager {
    pub fn new<T: Into<String>>(service_name: T) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    pub fn install<T: Into<String>>(&self, args: Vec<T>) {
        let service_binary_path = current_exe().unwrap();
        let file = Launchd::new(&self.service_name, &service_binary_path)
            .unwrap()
            .with_program_arguments(
                iter::once(service_binary_path.to_string_lossy().to_string())
                    .chain(args.into_iter().map(|a| a.into()))
                    .collect(),
            );
        let path =
            PathBuf::from("/Library/LaunchDaemons").join(format!("{}.plist", self.service_name));
        file.to_writer_xml(std::fs::File::create(path).unwrap())
            .unwrap();
        let path =
            PathBuf::from("/Library/LaunchDaemons").join(format!("{}.plist", self.service_name));
        self.run_launchctl(vec!["load", &path.to_string_lossy()]);
    }

    pub fn uninstall(&self) {
        let path =
            PathBuf::from("/Library/LaunchDaemons").join(format!("{}.plist", self.service_name));
        self.run_launchctl(vec!["unload", &path.to_string_lossy()]);
        fs::remove_file(path).unwrap();
    }

    pub fn start(&self) {
        self.run_launchctl(vec!["start", &self.service_name]);
    }

    pub fn stop(&self) {
        self.run_launchctl(vec!["stop", &self.service_name]);
    }

    pub fn query_status(&self) -> ServiceState {
        let output = self.run_launchctl(vec!["print", &format!("system/{}", self.service_name)]);
        if output.starts_with("Could not find service") {
            return ServiceState::NotInstalled;
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
            ServiceState::Started
        } else {
            ServiceState::Stopped
        }
    }

    pub fn is_installed(&self) -> bool {
        self.query_status() != ServiceState::NotInstalled
    }

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
}
