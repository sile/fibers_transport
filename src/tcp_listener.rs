use bytecodec::{Decode, Encode};
use factory::Factory;
use fibers::net::futures::Connected;
use fibers::net::streams::Incoming;
use fibers::net::TcpListener as RawTcpListener;
use futures::{Async, Future, Poll, Stream};
use std::net::SocketAddr;

use {Error, TcpTransporter, TcpTransporterBuilder};

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
    pub fn with_codec(encoder_factory: E, decoder_factory: D) -> Self {
        TcpListenerBuilder {
            encoder_factory,
            decoder_factory,
        }
    }

    pub fn listen(
        self,
        bind_addr: SocketAddr,
    ) -> impl Future<Item = TcpListener<E, D>, Error = Error> {
        track_err!(RawTcpListener::bind(bind_addr).map_err(Error::from))
            .map(move |listener| self.finish(listener))
    }

    pub fn finish(self, listener: RawTcpListener) -> TcpListener<E, D> {
        TcpListener {
            incoming: listener.incoming(),
            encoder_factory: self.encoder_factory,
            decoder_factory: self.decoder_factory,
            client_futures: Vec::new(),
        }
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

#[derive(Debug)]
pub struct TcpListener<E, D> {
    incoming: Incoming,
    encoder_factory: E,
    decoder_factory: D,
    client_futures: Vec<(SocketAddr, Connected)>,
}
impl<E, D> TcpListener<E, D>
where
    E: Factory + Default,
    D: Factory + Default,
    E::Item: Encode,
    D::Item: Decode,
{
    pub fn listen(bind_addr: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        TcpListenerBuilder::new().listen(bind_addr)
    }
}
impl<E, D> Stream for TcpListener<E, D>
where
    E: Factory,
    D: Factory,
    E::Item: Encode,
    D::Item: Decode,
{
    type Item = (SocketAddr, TcpTransporter<E::Item, D::Item>);
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        while let Async::Ready(client) = track!(self.incoming.poll().map_err(Error::from))? {
            if let Some((future, addr)) = client {
                self.client_futures.push((addr, future));
            } else {
                return Ok(Async::Ready(None));
            }
        }

        for i in 0..self.client_futures.len() {
            if let Async::Ready(stream) =
                track!(self.client_futures[i].1.poll().map_err(Error::from))?
            {
                let addr = self.client_futures.swap_remove(i).0;
                let encoder = self.encoder_factory.create();
                let decoder = self.decoder_factory.create();
                let transporter =
                    TcpTransporterBuilder::with_codec(encoder, decoder).finish(stream);
                return Ok(Async::Ready(Some((addr, transporter))));
            }
        }
        Ok(Async::NotReady)
    }
}
impl<E, D> From<RawTcpListener> for TcpListener<E, D>
where
    E: Factory + Default,
    D: Factory + Default,
    E::Item: Encode,
    D::Item: Decode,
{
    fn from(f: RawTcpListener) -> Self {
        TcpListenerBuilder::new().finish(f)
    }
}
