use daemon_slayer_core::config::Mergeable;

use super::EnvironmentVariable;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "config", derive(confique::Config))]
pub struct UserConfig {
    #[cfg_attr(feature="config",config(default=[]))]
    pub environment_variables: Vec<EnvironmentVariable>,
}

impl Mergeable for UserConfig {
    fn merge(user_config: Option<&Self>, app_config: &Self) -> Self {
        let mut environment_variables = vec![];
        if let Some(user_config) = user_config {
            environment_variables.extend_from_slice(&user_config.environment_variables);
        }

        environment_variables.extend_from_slice(&app_config.environment_variables);
        UserConfig {
            environment_variables,
        }
    }
}

impl UserConfig {
    #[cfg(feature = "cli")]
    pub fn pretty_printer(&self) -> daemon_slayer_core::cli::Printer {
        use owo_colors::OwoColorize;

        daemon_slayer_core::cli::Printer::default().with_multi_line(
            "Environment".cyan().to_string(),
            if self.environment_variables.is_empty() {
                vec!["N/A".dimmed().to_string()]
            } else {
                self.environment_variables
                    .iter()
                    .map(|e| format!("{}={}", e.name, e.value.bold()))
                    .collect()
            },
        )
    }
}
