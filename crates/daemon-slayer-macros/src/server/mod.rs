#[cfg(windows)]
mod windows_macros;

#[cfg(unix)]
mod unix_macros;

#[cfg(windows)]
pub mod platform {
    pub(crate) use super::windows_macros::*;
}

#[cfg(unix)]
pub mod platform {
    pub(crate) use super::unix_macros::*;
}
