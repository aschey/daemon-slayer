use enumflags2::{bitflags, BitFlags};

#[derive(Clone)]
pub enum Trustee {
    CurrentUser,
    Name(String),
}

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum ServiceAccess {
    QueryStatus,
    Start,
    Stop,
    PauseContinue,
    Interrogate,
    Delete,
    QueryConfig,
    ChangeConfig,
}

#[derive(Default, Clone)]
pub struct WindowsConfig {
    pub(crate) additional_access: Option<(Trustee, BitFlags<ServiceAccess>)>,
}

impl WindowsConfig {
    pub fn with_additional_access(
        mut self,
        trustee: Trustee,
        service_access: BitFlags<ServiceAccess>,
    ) -> Self {
        self.additional_access = Some((trustee, service_access));
        self
    }
}
