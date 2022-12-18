use eyre::Context;
use launchd::Launchd;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    configuration::{Builder, Level},
    Info, Manager, Result, State,
};

macro_rules! regex {
    ($name: ident, $re:literal $(,)?) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new($re).unwrap());
    };
}

regex!(STATE_RE, r"state = (\w+)");
regex!(PID_RE, r"pid = (\w+)");
regex!(AUTOSTART_RE, r"runatload = (\w+)");
regex!(EXIT_CODE_RE, r"last exit code = (\w+)");

#[derive(Clone)]
pub struct LaunchdServiceManager {
    configuration: Builder,
}

impl LaunchdServiceManager {
    pub(crate) fn from_builder(builder: Builder) -> Result<Self> {
        Ok(Self {
            configuration: builder,
        })
    }

    fn run_launchctl(&self, arguments: Vec<&str>) -> Result<String> {
        self.run_cmd("launchctl", arguments)
    }

    fn run_cmd(&self, command: &str, arguments: Vec<&str>) -> Result<String> {
        let output = Command::new(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(arguments)
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

    fn service_target(&self) -> Result<String> {
        match self.configuration.service_level {
            Level::System => Ok(format!("system/{}", self.name())),
            Level::User => {
                let id = self.run_cmd("id", vec!["-u"])?;
                Ok(format!("gui/{id}/{}", self.name()))
            }
        }
    }

    fn user_agent_dir(&self) -> Result<PathBuf> {
        let user_dirs = directories::UserDirs::new().ok_or("User dirs not found")?;
        let home_dir = user_dirs.home_dir();
        Ok(home_dir.join("Library").join("LaunchAgents"))
    }

    fn get_plist_path(&self) -> Result<PathBuf> {
        let path = match self.configuration.service_level {
            Level::System => PathBuf::from("/Library/LaunchDaemons"),
            Level::User => self.user_agent_dir()?,
        };
        if !path.exists() {
            std::fs::create_dir_all(&path).wrap_err("Error creating plist path")?;
        }
        Ok(path.join(format!("{}.plist", self.name())))
    }

    fn get_match_or_default<'a>(&self, re: &Regex, output: &'a str) -> Option<&'a str> {
        let captures = re.captures(output);
        let capture = captures?.get(1)?;
        let str_cap: &'a str = capture.as_str();
        Some(str_cap)
    }

    fn update_autostart(&mut self) -> Result<()> {
        let was_started = self.info()?.state == State::Started;
        if was_started {
            self.stop()?;
        }

        let mut config = Launchd::from_file(self.get_plist_path()?)?;
        config = config.with_run_at_load(self.configuration.autostart);
        let path = self.get_plist_path()?;
        self.run_launchctl(vec!["unload", &path.to_string_lossy()])?;
        config
            .to_writer_xml(std::fs::File::create(&path).wrap_err("Error creating config file")?)
            .wrap_err("Error writing config file")?;
        self.run_launchctl(vec!["load", &path.to_string_lossy()])?;

        if was_started {
            self.start()?;
        } else {
            self.stop()?;
        }

        Ok(())
    }
}

impl Manager for LaunchdServiceManager {
    fn on_configuration_changed(&mut self) -> Result<()> {
        let snapshot = self.configuration.user_configuration.snapshot();
        self.configuration.user_configuration.reload();
        let current = self.configuration.user_configuration.load();
        if current.environment_variables != snapshot.environment_variables {
            self.reload_configuration()?;
        }
        Ok(())
    }

    fn reload_configuration(&self) -> Result<()> {
        let current_state = self.info()?.state;
        self.stop()?;
        let path = self.get_plist_path()?;
        self.run_launchctl(vec!["unload", &path.to_string_lossy()])?;
        self.install()?;
        if current_state == State::Started {
            self.start()?;
        }
        Ok(())
    }

    fn install(&self) -> Result<()> {
        let mut file = Launchd::new(self.name(), &self.configuration.program)
            .wrap_err("Error creating config")?
            .with_program_arguments(
                self.configuration
                    .full_arguments_iter()
                    .map(|a| a.to_owned())
                    .collect(),
            )
            .with_run_at_load(self.configuration.autostart);

        let vars = self.configuration.environment_variables();
        for (key, value) in vars {
            file = file.with_environment_variable(key, value);
        }

        let path = self.get_plist_path()?;
        file.to_writer_xml(std::fs::File::create(&path).wrap_err("Error creating config file")?)
            .wrap_err(format!("Error writing config file {path:?}"))?;

        self.run_launchctl(vec!["load", &path.to_string_lossy()])?;

        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let path = self.get_plist_path()?;
        self.run_launchctl(vec!["unload", &path.to_string_lossy()])?;
        if path.exists() {
            fs::remove_file(&path).wrap_err(format!("Error removing config file {path:?}"))?;
        }

        Ok(())
    }

    fn start(&self) -> Result<()> {
        self.run_launchctl(vec!["start", &self.name()])?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.run_launchctl(vec!["stop", &self.name()])?;
        Ok(())
    }

    fn enable_autostart(&mut self) -> Result<()> {
        self.configuration.autostart = true;
        self.update_autostart()?;
        Ok(())
    }

    fn disable_autostart(&mut self) -> Result<()> {
        self.configuration.autostart = true;
        self.update_autostart()?;
        Ok(())
    }

    fn restart(&self) -> Result<()> {
        self.run_launchctl(vec!["kickstart", "-k", &self.service_target()?])?;
        Ok(())
    }

    fn info(&self) -> Result<Info> {
        let output = self.run_launchctl(vec!["print", &self.service_target()?])?;
        if output.contains("could not find service") {
            return Ok(Info {
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                last_exit_code: None,
            });
        }
        let state = match self.get_match_or_default(&STATE_RE, &output) {
            Some("running") => State::Started,
            _ => State::Stopped,
        };

        let pid = self
            .get_match_or_default(&PID_RE, &output)
            .map(|pid| pid.parse::<u32>().unwrap_or(0));

        let autostart = match self.get_match_or_default(&AUTOSTART_RE, &output) {
            Some("1") => Some(true),
            _ => Some(false),
        };

        let last_exit_code = self
            .get_match_or_default(&EXIT_CODE_RE, &output)
            .map(|code| code.parse::<i32>().unwrap_or(0));

        Ok(Info {
            state,
            pid,
            autostart,
            last_exit_code,
        })
    }

    fn display_name(&self) -> &str {
        self.configuration.display_name()
    }

    fn name(&self) -> String {
        self.configuration.label.qualified_name()
    }

    fn arguments(&self) -> &Vec<String> {
        &self.configuration.arguments
    }

    fn description(&self) -> &str {
        &self.configuration.description
    }
}
