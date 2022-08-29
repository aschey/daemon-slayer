use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use eyre::Context;
use launchd::Launchd;

use crate::{
    service_builder::{ServiceBuilder, ServiceLevel},
    service_manager::{Result, ServiceManager},
    service_status::ServiceStatus,
};

pub struct Manager {
    config: ServiceBuilder,
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
        let out = String::from_utf8(out_bytes)
            .wrap_err("Error reading output")?
            .trim()
            .to_lowercase()
            .to_owned();
        Ok(out)
    }

    fn user_agent_dir(&self) -> Result<PathBuf> {
        let user_dirs = directories::UserDirs::new().ok_or("User dirs not found")?;
        let home_dir = user_dirs.home_dir();
        Ok(home_dir.join("Library").join("LaunchAgents"))
    }

    fn get_plist_path(&self) -> Result<PathBuf> {
        let path = match self.config.service_level {
            ServiceLevel::System => PathBuf::from("/Library/LaunchDaemons"),
            ServiceLevel::User => self.user_agent_dir()?,
        };
        Ok(path.join(format!("{}.plist", self.config.name)))
    }
}

impl ServiceManager for Manager {
    fn builder(name: impl Into<String>) -> ServiceBuilder {
        ServiceBuilder::new(name)
    }

    fn new(name: impl Into<String>) -> Result<Self> {
        ServiceBuilder::new(name).build()
    }

    fn from_builder(builder: ServiceBuilder) -> Result<Self> {
        Ok(Self { config: builder })
    }

    fn install(&self) -> Result<()> {
        let file = Launchd::new(&self.config.name, &self.config.program)
            .wrap_err("Error creating config")?
            .with_program_arguments(self.config.full_args_iter().map(|a| a.to_owned()).collect());

        let path = self.get_plist_path()?;
        file.to_writer_xml(std::fs::File::create(&path).wrap_err("Error creating config file")?)
            .wrap_err("Error writing config file")?;

        self.run_launchctl(vec!["load", &path.to_string_lossy()])?;
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let path = self.get_plist_path()?;
        self.run_launchctl(vec!["unload", &path.to_string_lossy()])?;
        if path.exists() {
            fs::remove_file(path).wrap_err("Error removing config file")?;
        }

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
        if output.contains("could not find service") {
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
        if s.len() == 0 {
            return Ok(ServiceStatus::Stopped);
        }
        if s[0].trim() == "running" {
            Ok(ServiceStatus::Started)
        } else {
            Ok(ServiceStatus::Stopped)
        }
    }
}
