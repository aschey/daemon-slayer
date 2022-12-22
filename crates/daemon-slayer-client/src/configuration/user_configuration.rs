use super::EnvironmentVariable;
use daemon_slayer_core::config::Mergeable;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "config", derive(confique::Config))]
pub struct UserConfiguration {
    #[cfg_attr(feature="config",config(default=[]))]
    pub environment_variables: Vec<EnvironmentVariable>,
}

impl Mergeable for UserConfiguration {
    fn merge(user_config: Option<&Self>, app_config: &Self) -> Self {
        let mut environment_variables = vec![];
        if let Some(user_config) = user_config {
            environment_variables.extend_from_slice(&user_config.environment_variables);
        }

        environment_variables.extend_from_slice(&app_config.environment_variables);
        UserConfiguration {
            environment_variables,
        }
    }
}
