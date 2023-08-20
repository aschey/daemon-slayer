mod service_access;
mod trustee;

use enumflags2::BitFlags;
use owo_colors::OwoColorize;
pub use service_access::*;
pub use trustee::*;

#[derive(Default, Clone, Debug)]
pub struct WindowsConfig {
    pub(crate) additional_access: Vec<(Trustee, BitFlags<ServiceAccess>)>,
}

impl WindowsConfig {
    pub fn with_additional_access(
        mut self,
        trustee: Trustee,
        service_access: BitFlags<ServiceAccess>,
    ) -> Self {
        self.additional_access.push((trustee, service_access));
        self
    }

    #[cfg(feature = "cli")]
    pub fn pretty_printer(&self) -> daemon_slayer_core::cli::Printer {
        daemon_slayer_core::cli::Printer::default().with_multi_line(
            "Access".cyan().to_string(),
            if self.additional_access.is_empty() {
                vec!["N/A".dimmed().to_string()]
            } else {
                self.additional_access
                    .iter()
                    .map(|a| format!("{}: {}", a.0, a.1.to_string().bold()))
                    .collect()
            },
        )
    }
}
