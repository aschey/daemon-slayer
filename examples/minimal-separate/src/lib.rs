use daemon_slayer::core::Label;

pub fn label() -> Label {
    "com.example.daemon_slayer_minimal_separate"
        .parse()
        .expect("Should parse the label")
}
