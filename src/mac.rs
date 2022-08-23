use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use eyre::Context;
use launchd::Launchd;

use crate::{
    service_config::ServiceConfig,
    service_manager::{Result, ServiceManager},
    service_status::ServiceStatus,
};

pub struct Manager {
    config: ServiceConfig,
}

impl Manager {
    fn run_launchctl(&self, args: Vec<&str>) -> Result<String> {
        let output = Command::new("launchctl")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(args)
            .output()
            .wrap_err("Error running launchctl")?;

        let out_bytes = if output.status.success() {
            output.stdout
        } else {
            output.stderr
        };
        let out = String::from_utf8(out_bytes).wrap_err("Error reading output")?;
        Ok(out)
    }

    fn get_plist_path(&self) -> PathBuf {
        PathBuf::from("/Library/LaunchDaemons").join(format!("{}.plist", self.config.name))
    }
}

impl ServiceManager for Manager {
    fn new(config: ServiceConfig) -> Result<Self> {
        Ok(Self { config })
    }

    fn install(&self) -> Result<()> {
        let file = Launchd::new(&self.config.name, &self.config.program)
            .wrap_err("Error creating config")?
            .with_program_arguments(self.config.args_iter().map(|a| a.to_owned()).collect());

        let path = self.get_plist_path();
        file.to_writer_xml(std::fs::File::create(&path).wrap_err("Error creating config file")?)
            .wrap_err("Error writing config file")?;

        self.run_launchctl(vec!["load", &path.to_string_lossy()])?;
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let path = self.get_plist_path();
        self.run_launchctl(vec!["unload", &path.to_string_lossy()])?;
        fs::remove_file(path).wrap_err("Error removing config file")?;
        Ok(())
    }

    fn start(&self) -> Result<()> {
        self.run_launchctl(vec!["start", &self.config.name])?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.run_launchctl(vec!["stop", &self.config.name])?;
        Ok(())
    }

    fn query_status(&self) -> Result<ServiceStatus> {
        let output = self.run_launchctl(vec!["print", &format!("system/{}", self.config.name)])?;
        if output.starts_with("Could not find service") {
            return Ok(ServiceStatus::NotInstalled);
        }
        let s = output
            .split('\n')
            .into_iter()
            .filter(|line| line.trim().starts_with("state"))
            .map(|line| {
                line.split('=')
                    .collect::<Vec<_>>()
                    .get(1)
                    .unwrap_or(&"")
                    .to_string()
            })
            .collect::<Vec<_>>();

        if s[0].trim() == "running" {
            Ok(ServiceStatus::Started)
        } else {
            Ok(ServiceStatus::Stopped)
        }
    }
}
