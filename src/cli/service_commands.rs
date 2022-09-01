pub(crate) struct ServiceCommands;

impl ServiceCommands {
    pub(crate) const INSTALL: &'static str = "install";
    pub(crate) const UNINSTALL: &'static str = "uninstall";
    pub(crate) const RUN: &'static str = "run";
    #[cfg(feature = "direct")]
    pub(crate) const DIRECT: &'static str = "direct";
    pub(crate) const STATUS: &'static str = "status";
    pub(crate) const START: &'static str = "start";
    pub(crate) const STOP: &'static str = "stop";
}
