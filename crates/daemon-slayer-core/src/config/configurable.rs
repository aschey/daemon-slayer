use std::{ops::Deref, sync::Arc};

use arc_swap::{
    access::{Access, DynAccess, DynGuard, Map},
    ArcSwap, ArcSwapAny,
};

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
        CachedConfig::nonreloadable(self.clone())
    }
}

pub trait Mergeable {
    fn merge(user_config: Option<&Self>, app_config: &Self) -> Self;
}

#[derive(Clone, Default)]
pub struct CachedConfig<T: Mergeable + Clone + Default> {
    inner: Option<Arc<Box<dyn DynAccess<T> + Send + Sync>>>,
    cache: Option<T>,
    explicit: T,
}

impl<T: Mergeable + Clone + Default> CachedConfig<T> {
    fn new(inner: Box<dyn DynAccess<T> + Send + Sync>) -> Self {
        Self {
            cache: Some(inner.load().clone()),
            inner: Some(Arc::new(inner)),
            explicit: T::default(),
        }
    }

    fn nonreloadable(inner: T) -> Self {
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
