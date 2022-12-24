use daemon_slayer::core::Label;

pub fn label() -> Label {
    "com.example.daemonslayerminimalseparate"
        .parse()
        .expect("Should parse the label")
}
