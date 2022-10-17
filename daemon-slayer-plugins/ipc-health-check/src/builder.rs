pub struct Builder {
    pub(crate) sock_path: String,
}

impl Builder {
    pub fn new(app_name: String) -> Self {
        #[cfg(unix)]
        let sock_path = format!("/tmp/{app_name}health.sock");
        #[cfg(windows)]
        let sock_path = format!("\\\\.\\pipe\\{app_name}health");
        Self { sock_path }
    }
}
