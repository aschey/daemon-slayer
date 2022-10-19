use crate::{config::Builder, Info, Manager, Result, State};
use eyre::Context;
use systemd_client::{
    create_unit_configuration_file, create_user_unit_configuration_file,
    delete_unit_configuration_file, delete_user_unit_configuration_file,
    manager::{self, SystemdManagerProxyBlocking},
    service, unit, InstallConfiguration, NotifyAccess, ServiceConfiguration, ServiceType,
    ServiceUnitConfiguration, UnitActiveStateType, UnitConfiguration, UnitFileState,
    UnitLoadStateType, UnitSubStateType,
};

#[derive(Clone)]
pub struct ServiceManager {
    config: Builder,
    client: SystemdManagerProxyBlocking<'static>,
}

impl ServiceManager {
    fn service_file_name(&self) -> String {
        format!("{}.service", self.config.name)
    }

    fn update_autostart(&self) -> Result<()> {
        if self.config.autostart {
            self.client
                .enable_unit_files(&[&self.service_file_name()], false, true)?;
        } else {
            self.client
                .disable_unit_files(&[&self.service_file_name()], false)?;
        }
        Ok(())
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
        let client = if builder.is_user() {
            manager::build_blocking_user_proxy()
        } else {
            manager::build_blocking_proxy()
        }
        .wrap_err("Error creating systemd proxy")?;
        Ok(Self {
            config: builder,
            client,
        })
    }

    fn install(&self) -> Result<()> {
        let mut unit_config = UnitConfiguration::builder().description(&self.config.description);
        for after in &self.config.systemd_config.after {
            unit_config = unit_config.after(after);
        }

        let mut service_config = ServiceConfiguration::builder()
            .exec_start(self.config.full_args_iter().map(String::as_ref).collect())
            .ty(ServiceType::Notify)
            .notify_access(NotifyAccess::Main);

        for (key, value) in &self.config.env_vars {
            service_config = service_config.env(key, value);
        }

        let mut svc_unit_builder = ServiceUnitConfiguration::builder()
            .unit(unit_config)
            .service(service_config);

        if self.config.is_user() {
            svc_unit_builder = svc_unit_builder
                .install(InstallConfiguration::builder().wanted_by("default.target"));
        }

        let svc_unit_literal = format!("{}", svc_unit_builder.build());

        if self.config.is_user() {
            create_user_unit_configuration_file(
                &self.service_file_name(),
                svc_unit_literal.as_bytes(),
            )
        } else {
            create_unit_configuration_file(&self.service_file_name(), svc_unit_literal.as_bytes())
        }
        .wrap_err("Error creating systemd config file")?;

        self.update_autostart()?;

        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        if self.config.is_user() {
            delete_user_unit_configuration_file(&self.service_file_name())
        } else {
            delete_unit_configuration_file(&self.service_file_name())
        }
        .wrap_err("Error removing systemd config file")?;
        Ok(())
    }

    fn start(&self) -> Result<()> {
        self.client
            .start_unit(&self.service_file_name(), "replace")
            .wrap_err("Error starting systemd unit")?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if self.info()?.state == State::Started {
            self.client
                .stop_unit(&self.service_file_name(), "replace")
                .wrap_err("Error stopping systemd unit")?;
        }

        Ok(())
    }

    fn restart(&self) -> Result<()> {
        if self.info()?.state == State::Started {
            self.client
                .restart_unit(&self.service_file_name(), "replace")
                .wrap_err("Error stopping systemd unit")?;
        } else {
            self.start()?;
        }
        Ok(())
    }

    fn info(&self) -> Result<Info> {
        self.client
            .reload()
            .wrap_err("Error reloading systemd units")?;

        self.client
            .reset_failed()
            .wrap_err("Error reseting failed unit state")?;

        let svc_unit_path = self
            .client
            .load_unit(&self.service_file_name())
            .wrap_err("Error loading systemd unit")?;

        let unit_client = if self.config.is_user() {
            unit::build_blocking_user_proxy(svc_unit_path.clone())
        } else {
            unit::build_blocking_proxy(svc_unit_path.clone())
        }
        .wrap_err("Error creating unit client")?;

        let unit_props = unit_client
            .get_properties()
            .wrap_err("Error getting unit properties")?;

        let state = match (
            unit_props.load_state,
            unit_props.active_state,
            unit_props.sub_state,
        ) {
            (UnitLoadStateType::Loaded, UnitActiveStateType::Active, UnitSubStateType::Running) => {
                State::Started
            }
            (UnitLoadStateType::NotFound, _, _) => State::NotInstalled,
            _ => State::Stopped,
        };

        let service_client = if self.config.is_user() {
            service::build_blocking_user_proxy(svc_unit_path)
        } else {
            service::build_blocking_proxy(svc_unit_path)
        }
        .wrap_err("Error creating service client")?;

        let service_props = service_client
            .get_properties()
            .wrap_err("Error getting service properties")?;

        let autostart = match (&state, unit_props.unit_file_state) {
            (State::NotInstalled, _) => None,
            (_, UnitFileState::Enabled | UnitFileState::EnabledRuntime | UnitFileState::Static) => {
                Some(true)
            }
            _ => Some(false),
        };

        let pid = if state == State::Started {
            Some(service_props.exec_main_pid)
        } else {
            None
        };

        let last_exit_code = if state == State::NotInstalled {
            None
        } else {
            Some(service_props.exec_main_status)
        };

        Ok(Info {
            pid,
            state,
            autostart,
            last_exit_code,
        })
    }

    fn set_autostart_enabled(&mut self, enabled: bool) -> Result<()> {
        self.config.autostart = enabled;
        self.update_autostart()?;
        Ok(())
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
}
