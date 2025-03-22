use std::io;
use std::sync::Arc;

pub use arc_swap;
use arc_swap::ArcSwap;
use arc_swap::access::{DynAccess, Map};
use async_trait::async_trait;
use derivative::Derivative;
use dyn_clonable::clonable;

pub trait Accessor<T: Mergeable + Clone + Default> {
    fn access(&self) -> CachedConfig<T>;
}

impl<A, T> Accessor<T> for Arc<ArcSwap<A>>
where
    Self: Clone,
    T: Mergeable + Clone + Default + 'static,
    A: AsRef<T> + Send + Sync + 'static,
{
    fn access(&self) -> CachedConfig<T> {
        CachedConfig::new(Box::new(Map::new(self.clone(), |conf: &A| conf.as_ref())))
    }
}

impl<T> Accessor<T> for T
where
    T: Mergeable + Clone + Default + 'static,
{
    fn access(&self) -> CachedConfig<T> {
        CachedConfig::non_reloadable(self.clone())
    }
}

pub trait Mergeable {
    fn merge(user_config: Option<&Self>, app_config: &Self) -> Self;
}

#[derive(Clone, Default, Derivative)]
#[derivative(Debug)]
pub struct CachedConfig<T: Mergeable + Clone + Default + 'static> {
    #[derivative(Debug = "ignore")]
    inner: Option<Arc<dyn DynAccess<T> + Send + Sync>>,
    cache: Option<T>,
    explicit: T,
}

impl<T: Mergeable + Clone + Default> CachedConfig<T> {
    fn new(inner: impl DynAccess<T> + Send + Sync + 'static) -> Self {
        Self {
            cache: Some(inner.load().clone()),
            inner: Some(Arc::new(inner)),
            explicit: T::default(),
        }
    }

    fn non_reloadable(inner: T) -> Self {
        Self {
            cache: Some(inner.clone()),
            inner: None,
            explicit: inner,
        }
    }

    pub fn edit(&mut self) -> &mut T {
        &mut self.explicit
    }

    pub fn load(&self) -> T {
        let inner = &self.inner.as_ref().map(|i| i.load().clone());
        T::merge(inner.as_ref(), &self.explicit)
    }

    pub fn snapshot(&self) -> T {
        T::merge(self.cache.as_ref(), &self.explicit)
    }

    pub fn reload(&mut self) {
        if let Some(inner) = &self.inner {
            self.cache = Some(inner.load().clone());
        }
    }
}

#[clonable]
#[async_trait]
pub trait ConfigWatcher: Clone + Send + Sync + 'static {
    async fn on_config_changed(&mut self) -> io::Result<()>;
}

#[async_trait]
impl<T> ConfigWatcher for Box<T>
where
    T: ConfigWatcher + Clone + Send + Sync + 'static,
{
    async fn on_config_changed(&mut self) -> io::Result<()> {
        (**self).on_config_changed().await
    }
}
