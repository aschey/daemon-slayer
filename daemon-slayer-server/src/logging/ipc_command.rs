#[derive(Debug)]
pub(crate) enum IpcCommand {
    Flush,
    Write(Vec<u8>),
}
