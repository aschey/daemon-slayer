use daemon_slayer::core::{CommandArg, Label};

pub fn label() -> Label {
    "com.example.daemon_slayer_containerized"
        .parse()
        .expect("Should parse the label")
}

pub fn run_argument() -> CommandArg {
    "run".parse().expect("Should parse the command argument")
}
