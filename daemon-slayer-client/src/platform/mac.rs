use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use eyre::Context;
use launchd::Launchd;
use regex::Regex;

use crate::{Builder, Info, Level, Manager, Result, State};

pub struct ServiceManager {
    config: Builder,
}

impl ServiceManager {
    fn run_launchctl(&self, args: Vec<&str>) -> Result<String> {
        self.run_cmd("launchctl", args)
    }

    fn run_cmd(&self, command: &str, args: Vec<&str>) -> Result<String> {
        let output = Command::new(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(args)
            .output()
            .wrap_err("Error running command")?;

        let out_bytes = if output.status.success() {
            output.stdout
        } else {
            output.stderr
        };
        let out = String::from_utf8(out_bytes)
            .wrap_err("Error reading output")?
            .trim()
            .to_lowercase();
        Ok(out)
    }

    fn user_agent_dir(&self) -> Result<PathBuf> {
        let user_dirs = directories::UserDirs::new().ok_or("User dirs not found")?;
        let home_dir = user_dirs.home_dir();
        Ok(home_dir.join("Library").join("LaunchAgents"))
    }

    fn get_plist_path(&self) -> Result<PathBuf> {
        let path = match self.config.service_level {
            Level::System => PathBuf::from("/Library/LaunchDaemons"),
            Level::User => self.user_agent_dir()?,
        };
        if !path.exists() {
            std::fs::create_dir_all(&path).wrap_err("Error creating plist path")?;
        }
        Ok(path.join(format!("{}.plist", self.config.name)))
    }
}

impl Manager for ServiceManager {
    fn builder(name: impl Into<String>) -> Builder {
        Builder::new(name)
    }

    fn new(name: impl Into<String>) -> Result<Self> {
        Builder::new(name).build()
    }

    fn from_builder(builder: Builder) -> Result<Self> {
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

    fn restart(&self) -> Result<()> {
        todo!();
    }

    fn info(&self) -> Result<Info> {
        let output = match self.config.service_level {
            Level::System => {
                self.run_launchctl(vec!["print", &format!("system/{}", self.config.name)])?
            }
            Level::User => {
                let id = self.run_cmd("id", vec!["-u"])?;
                self.run_launchctl(vec!["print", &format!("gui/{id}/{}", self.config.name)])?
            }
        };

        if output.contains("could not find service") {
            return Ok(Info {
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                last_exit_code: None,
            });
        }

        let re = Regex::new(r"state = (\w+)").unwrap();

        let captures = re.captures(&output);
        let state = match captures {
            Some(captures) => match captures.get(1) {
                Some(state_capture) => match state_capture.as_str() {
                    "running" => State::Started,
                    _ => State::Stopped,
                },
                None => State::Stopped,
            },
            None => State::Stopped,
        };

        Ok(Info {
            state,
            pid: None,
            autostart: None,
            last_exit_code: None,
        })
    }

    fn display_name(&self) -> &str {
        &self.config.display_name
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn args(&self) -> &Vec<String> {
        &self.config.args
    }

    fn description(&self) -> &str {
        &self.config.description
    }

    fn set_autostart_enabled(&mut self, enabled: bool) -> Result<()> {
        todo!()
    }
}
