pub trait Configurable {
    type UserConfig;

    fn with_user_config(self, config: Self::UserConfig) -> Self;
}
