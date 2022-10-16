#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ServiceCommand {
    #[cfg(feature = "client")]
    Install,
    #[cfg(feature = "client")]
    Uninstall,
    #[cfg(feature = "server")]
    Run,
    #[cfg(feature = "server")]
    Direct,
    #[cfg(feature = "client")]
    Info,
    #[cfg(feature = "client")]
    Start,
    #[cfg(feature = "client")]
    Stop,
    #[cfg(feature = "client")]
    Restart,
    #[cfg(feature = "client")]
    Enable,
    #[cfg(feature = "client")]
    Disable,
    #[cfg(feature = "client")]
    Pid,
    #[cfg(feature = "client")]
    Health,
    #[cfg(all(feature = "client", feature = "console"))]
    Console,
}

impl From<ServiceCommand> for &str {
    fn from(source: ServiceCommand) -> Self {
        match source {
            #[cfg(feature = "client")]
            ServiceCommand::Install => "install",
            #[cfg(feature = "client")]
            ServiceCommand::Uninstall => "uninstall",
            #[cfg(feature = "server")]
            ServiceCommand::Run => "run",
            #[cfg(feature = "server")]
            ServiceCommand::Direct => "direct",
            #[cfg(feature = "client")]
            ServiceCommand::Info => "info",
            #[cfg(feature = "client")]
            ServiceCommand::Start => "start",
            #[cfg(feature = "client")]
            ServiceCommand::Stop => "stop",
            #[cfg(feature = "client")]
            ServiceCommand::Restart => "restart",
            #[cfg(feature = "client")]
            ServiceCommand::Enable => "enable",
            #[cfg(feature = "client")]
            ServiceCommand::Disable => "disable",
            #[cfg(feature = "client")]
            ServiceCommand::Pid => "pid",
            #[cfg(feature = "client")]
            ServiceCommand::Health => "health",
            #[cfg(all(feature = "client", feature = "console"))]
            ServiceCommand::Console => "console",
        }
    }
}

impl From<ServiceCommand> for String {
    fn from(source: ServiceCommand) -> Self {
        let s: &str = source.into();
        s.to_owned()
    }
}

impl ToString for ServiceCommand {
    fn to_string(&self) -> String {
        self.clone().into()
    }
}
