use bytecodec::{Decode, Encode};
use factory::Factory;
use fibers::net::futures::Connected;
use fibers::net::streams::Incoming;
use fibers::net::TcpListener as RawTcpListener;
use futures::{Async, Future, Poll, Stream};
use std::net::SocketAddr;

use {Error, Result, TcpTransporter, TcpTransporterBuilder};

/// [`TcpListener`] builder.
///
/// [`TcpListener`]: ./struct.TcpListener.html
#[derive(Debug)]
pub struct TcpListenerBuilder<E, D> {
    encoder_factory: E,
    decoder_factory: D,
}
impl<E, D> TcpListenerBuilder<E, D>
where
    E: Factory + Default,
    D: Factory + Default,
    E::Item: Encode,
    D::Item: Decode,
{
    /// Makes a new `TcpListenerBuilder` instance with the default settings.
    pub fn new() -> Self {
        Self::with_codec(E::default(), D::default())
    }
}
impl<E, D> TcpListenerBuilder<E, D>
where
    E: Factory,
    D: Factory,
    E::Item: Encode,
    D::Item: Decode,
{
    /// Makes a new `TcpListenerBuilder` instance with the given encoder and decoder factories.
    pub fn with_codec(encoder_factory: E, decoder_factory: D) -> Self {
        TcpListenerBuilder {
            encoder_factory,
            decoder_factory,
        }
    }

    /// Builds a new `TcpListener` instance from the given `RawTcpListener`.
    pub fn finish(self, listener: RawTcpListener) -> Result<TcpListener<E, D>> {
        let local_addr = track!(listener.local_addr().map_err(Error::from))?;
        Ok(TcpListener {
            incoming: listener.incoming(),
            local_addr,
            encoder_factory: self.encoder_factory,
            decoder_factory: self.decoder_factory,
            client_futures: Vec::new(),
        })
    }

    /// Builds a new `TcpListener` instance that binds to and listens in the given address.
    pub fn listen(
        self,
        bind_addr: SocketAddr,
    ) -> impl Future<Item = TcpListener<E, D>, Error = Error> {
        track_err!(RawTcpListener::bind(bind_addr).map_err(Error::from))
            .and_then(move |listener| track!(self.finish(listener)))
    }
}
impl<E, D> Default for TcpListenerBuilder<E, D>
where
    E: Factory + Default,
    D: Factory + Default,
    E::Item: Encode,
    D::Item: Decode,
{
    fn default() -> Self {
        Self::new()
    }
}

/// TCP listener.
#[must_use = "streams do nothing unless polled"]
#[derive(Debug)]
pub struct TcpListener<E, D> {
    incoming: Incoming,
    local_addr: SocketAddr,
    encoder_factory: E,
    decoder_factory: D,
    client_futures: Vec<Connected>,
}
impl<E, D> TcpListener<E, D>
where
    E: Factory + Default,
    D: Factory + Default,
    E::Item: Encode,
    D::Item: Decode,
{
    /// Makes a new `TcpListener` instance that binds to and listens in the given address.
    ///
    /// This is equivalent to `TcpListenerBuilder::new().listen(bind_addr)`.
    pub fn listen(bind_addr: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        TcpListenerBuilder::new().listen(bind_addr)
    }

    /// Returns the address on which the listener is listening.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}
impl<E, D> Stream for TcpListener<E, D>
where
    E: Factory,
    D: Factory,
    E::Item: Encode,
    D::Item: Decode,
{
    type Item = TcpTransporter<E::Item, D::Item>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        while let Async::Ready(client) = track!(self.incoming.poll().map_err(Error::from))? {
            if let Some((future, _)) = client {
                self.client_futures.push(future);
            } else {
                return Ok(Async::Ready(None));
            }
        }

        for i in 0..self.client_futures.len() {
            if let Async::Ready(stream) =
                track!(self.client_futures[i].poll().map_err(Error::from))?
            {
                self.client_futures.swap_remove(i);
                let encoder = self.encoder_factory.create();
                let decoder = self.decoder_factory.create();
                let transporter =
                    track!(TcpTransporterBuilder::with_codec(encoder, decoder).finish(stream))?;
                return Ok(Async::Ready(Some(transporter)));
            }
        }
        Ok(Async::NotReady)
    }
}
