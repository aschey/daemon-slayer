#[cfg(feature = "async-tokio")]
mod async_impl;
#[cfg(feature = "blocking")]
mod blocking_impl;
mod event_service;
mod event_store;
mod receiver;
mod service;

#[cfg(feature = "async-tokio")]
pub use crate::{
    async_impl::*, event_service::EventServiceAsync as EventService,
    event_store::EventStoreAsync as EventStore, receiver::ReceiverAsync as Receiver,
    service::ServiceAsync as Service,
};

#[cfg(feature = "blocking")]
pub mod blocking {
    pub use crate::{
        blocking_impl::*, event_service::EventServiceSync as EventService,
        event_store::EventStoreSync as EventStore, receiver::ReceiverSync as Receiver,
        service::ServiceSync as Service,
    };
}
