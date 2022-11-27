use std::sync::Arc;

use arc_swap::{
    access::{Access, DynAccess, Map},
    ArcSwap,
};

pub trait Configurable {
    type UserConfig;
    fn with_user_config(self, config: Box<dyn DynAccess<Self::UserConfig> + Send + Sync>) -> Self;
}
