use enumflags2::BitFlags;
mod service_access;
pub use service_access::*;
mod trustee;
pub use trustee::*;

#[derive(Default, Clone, Debug)]
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
