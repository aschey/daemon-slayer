use enumflags2::bitflags;

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
