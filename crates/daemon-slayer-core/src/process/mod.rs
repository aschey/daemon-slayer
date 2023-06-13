use crate::Label;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub mod platform {
    #[cfg(target_os = "linux")]
    pub use super::linux::*;
    #[cfg(target_os = "macos")]
    pub use super::macos::*;
    #[cfg(windows)]
    pub use super::windows::*;
}

pub fn get_admin_var(label: &Label) -> String {
    format!("{}_ADMIN", label.application.to_ascii_uppercase())
}
