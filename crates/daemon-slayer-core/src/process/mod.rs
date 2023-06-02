use crate::Label;

#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(windows)]
pub(crate) mod windows;

pub fn get_spawn_interactive_var(label: &Label) -> String {
    format!(
        "{}_SPAWN_INTERACTIVE",
        label.application.to_ascii_uppercase()
    )
}
