pub(crate) struct ServiceCommands;

impl ServiceCommands {
    #[cfg(feature = "client")]
    pub(crate) const INSTALL: &'static str = "install";
    #[cfg(feature = "client")]
    pub(crate) const UNINSTALL: &'static str = "uninstall";
    #[cfg(feature = "server")]
    pub(crate) const RUN: &'static str = "run";
    #[cfg(all(feature = "server", feature = "direct"))]
    pub(crate) const DIRECT: &'static str = "direct";
    #[cfg(feature = "client")]
    pub(crate) const STATUS: &'static str = "status";
    #[cfg(feature = "client")]
    pub(crate) const START: &'static str = "start";
    #[cfg(feature = "client")]
    pub(crate) const STOP: &'static str = "stop";
    #[cfg(all(feature = "client", feature = "console"))]
    pub(crate) const CONSOLE: &'static str = "console";
}
