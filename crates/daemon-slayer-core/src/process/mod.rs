use crate::Label;

#[cfg(windows)]
pub(crate) mod windows;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

pub fn get_spawn_interactive_var(label: &Label) -> String {
    format!(
        "{}_SPAWN_INTERACTIVE",
        label.application.to_ascii_uppercase()
    )
}
