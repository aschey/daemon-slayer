use eyre::Context;
use systemd_client::{
    create_unit_configuration_file, delete_unit_configuration_file,
    manager::{self, SystemdManagerProxyBlocking},
    unit, ServiceConfiguration, ServiceUnitConfiguration, UnitActiveStateType, UnitConfiguration,
    UnitLoadStateType, UnitSubStateType,
};

use crate::{
    service_config::ServiceConfig,
    service_manager::{Result, ServiceManager},
    service_status::ServiceStatus,
};

pub struct Manager {
    config: ServiceConfig,
    client: SystemdManagerProxyBlocking<'static>,
}

impl Manager {
    fn service_file_name(&self) -> String {
        format!("{}.service", self.config.name)
    }
}
impl ServiceManager for Manager {
    fn new(config: ServiceConfig) -> Result<Self> {
        let client = manager::build_blocking_proxy().wrap_err("Error creating systemd proxy")?;
        Ok(Self { config, client })
    }

    fn install(&self) -> Result<()> {
        let unit_config = UnitConfiguration::builder().description(&self.config.description);

        let service_config = ServiceConfiguration::builder()
            .exec_start(self.config.full_args_iter().map(|a| &a[..]).collect());
        let svc_unit = ServiceUnitConfiguration::builder()
            .unit(unit_config)
            .service(service_config)
            .build();
        let svc_unit_literal = format!("{}", svc_unit);

        create_unit_configuration_file(&self.service_file_name(), svc_unit_literal.as_bytes())
            .wrap_err("Error creating systemd config file")?;
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        delete_unit_configuration_file(&self.service_file_name())
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
        if self.query_status()? == ServiceStatus::Started {
            self.client
                .stop_unit(&self.service_file_name(), "replace")
                .wrap_err("Error stopping systemd unit")?;
        }

        Ok(())
    }

    fn query_status(&self) -> Result<ServiceStatus> {
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

        let unit_client =
            unit::build_blocking_proxy(svc_unit_path).wrap_err("Error creating unit client")?;

        let props = unit_client
            .get_properties()
            .wrap_err("Error getting properties")?;

        match (props.load_state, props.active_state, props.sub_state) {
            (UnitLoadStateType::Loaded, UnitActiveStateType::Active, UnitSubStateType::Running) => {
                Ok(ServiceStatus::Started)
            }
            (UnitLoadStateType::NotFound, _, _) => Ok(ServiceStatus::NotInstalled),
            _ => Ok(ServiceStatus::Stopped),
        }
    }
}
