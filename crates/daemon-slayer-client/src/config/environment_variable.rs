#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "config", derive(confique::Config, serde::Deserialize))]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}
