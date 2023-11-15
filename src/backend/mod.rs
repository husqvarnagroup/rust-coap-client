//! Backend module provides swappable CoAP client backends
// https://github.com/ryankurte/rust-coap-client
// Copyright 2021 ryan kurte <ryan@kurte.nz>

use async_trait::async_trait;
use coap_lite::Packet;
use futures::{Future, Stream};
use std::net::SocketAddr;
use std::pin::Pin;

use crate::RequestOptions;

#[cfg(feature = "backend-tokio")]
pub mod backend_tokio;

#[cfg(feature = "backend-tokio")]
pub use backend_tokio::{Tokio, TokioObserve};

pub trait Observer<E>: Stream<Item = Result<Packet, E>> + Send {
    /// Fetch the observer token
    fn token(&self) -> u32;
    /// Fetch the observer resource
    fn resource(&self) -> &str;
}

/// Generic transport trait for implementing CoAP client backends
#[async_trait]
pub trait Backend<E>: Send {
    type Observe: Observer<E>;

    fn request(
        &self,
        req: Packet,
        peer_addr: SocketAddr,
        opts: RequestOptions,
    ) -> Pin<Box<dyn Future<Output = Result<Packet, E>> + '_>>;

    async fn observe(
        &mut self,
        peer_addr: SocketAddr,
        resource: String,
        opts: RequestOptions,
    ) -> Result<Self::Observe, E>;

    async fn unobserve(&mut self, o: Self::Observe) -> Result<(), E>;
}
