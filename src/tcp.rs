use bytecodec::io::{BufferedIo, IoDecodeExt, IoEncodeExt};
use bytecodec::{Decode, Encode};
use fibers::net::TcpStream;
use futures::{Async, Future};
use std::collections::VecDeque;
use std::net::SocketAddr;

use base::Transport;
use {Error, PollRecv, PollSend, Result};

/// This trait indicates that the implementation implements TCP.
pub trait TcpTransport: Transport<PeerAddr = ()> {
    /// Returns the address of the connected peer.
    fn peer_addr(&self) -> SocketAddr;

    /// Returns the address to which the instance is bound.
    fn local_addr(&self) -> SocketAddr;
}

/// [`TcpTransporter`] builder.
///
/// [`TcpTransporter`]: ./struct.TcpTransporter.html
#[derive(Debug)]
pub struct TcpTransporterBuilder<E, D> {
    buf_size: usize,
    encoder: E,
    decoder: D,
}
impl<E, D> TcpTransporterBuilder<E, D>
where
    E: Encode + Default,
    D: Decode + Default,
{
    /// Makes a new `TcpTransporterBuilder` with the default settings.
    pub fn new() -> Self {
        Self::default()
    }
}
impl<E: Encode, D: Decode> TcpTransporterBuilder<E, D> {
    /// Makes a new `TcpTransporterBuilder` with the given encoder and decoder.
    pub fn with_codec(encoder: E, decoder: D) -> Self {
        TcpTransporterBuilder {
            buf_size: 8192,
            encoder,
            decoder,
        }
    }

    /// Sets the application level read/write buffer size of the resulting instance in byte.
    ///
    /// The default value is `8192`.
    pub fn buf_size(mut self, size: usize) -> Self {
        self.buf_size = size;
        self
    }

    /// Builds a `TcpTransporterBuilder` instance from the given `TcpStream`.
    pub fn finish(self, stream: TcpStream) -> Result<TcpTransporter<E, D>> {
        let _ = stream.set_nodelay(true);
        let peer_addr = track!(stream.peer_addr().map_err(Error::from))?;
        let local_addr = track!(stream.peer_addr().map_err(Error::from))?;
        Ok(TcpTransporter {
            stream: BufferedIo::new(stream, self.buf_size, self.buf_size),
            peer_addr,
            local_addr,
            encoder: self.encoder,
            decoder: self.decoder,
            outgoing_queue: VecDeque::new(),
        })
    }

    /// Builds a `TcpTransporterBuilder` instance by connecting to the specified peer.
    pub fn connect(
        self,
        peer: SocketAddr,
    ) -> impl Future<Item = TcpTransporter<E, D>, Error = Error> {
        TcpStream::connect(peer)
            .map_err(|e| track!(Error::from(e)))
            .and_then(move |stream| track!(self.finish(stream)))
    }
}
impl<E, D> Default for TcpTransporterBuilder<E, D>
where
    E: Encode + Default,
    D: Decode + Default,
{
    fn default() -> Self {
        Self::with_codec(E::default(), D::default())
    }
}

/// An implementation of [`Transport`] that uses TCP as the transport layer.
///
/// [`Transport`]: ./trait.Transport.html
#[derive(Debug)]
pub struct TcpTransporter<E: Encode, D: Decode> {
    stream: BufferedIo<TcpStream>,
    peer_addr: SocketAddr,
    local_addr: SocketAddr,
    decoder: D,
    encoder: E,
    outgoing_queue: VecDeque<E::Item>,
}
impl<E, D> TcpTransporter<E, D>
where
    E: Encode + Default,
    D: Decode + Default,
{
    /// Starts connecting to the given peer and
    /// will return a new `TcpTransporter` instance if the connect operation is succeeded.
    ///
    /// This is equivalent to `TcpTransporterBuilder::new().connect(peer)`.
    pub fn connect(peer: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        TcpTransporterBuilder::new().connect(peer)
    }

    /// Makes a new `TcpTransporter` instance from the given `TcpStream`.
    ///
    /// This is equivalent to `TcpTransporterBuilder::new().finish(stream)`.
    pub fn from_stream(stream: TcpStream) -> Result<Self> {
        TcpTransporterBuilder::new().finish(stream)
    }
}
impl<E: Encode, D: Decode> TcpTransporter<E, D> {
    /// Returns the number of unsent messages in the queue of the instance.
    pub fn message_queue_len(&self) -> usize {
        self.outgoing_queue.len() + if self.encoder.is_idle() { 0 } else { 1 }
    }

    /// Returns a reference to the TCP stream being used by the instance.
    pub fn stream_ref(&self) -> &TcpStream {
        self.stream.stream_ref()
    }

    /// Returns a mutable reference to the TCP stream being used by the instance.
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        self.stream.stream_mut()
    }

    /// Returns a reference to the decoder being used by the instance.
    pub fn decoder_ref(&self) -> &D {
        &self.decoder
    }

    /// Returns a mutable reference to the decoder being used by the instance.
    pub fn decoder_mut(&mut self) -> &mut D {
        &mut self.decoder
    }

    /// Returns a reference to the encoder being used by the instance.
    pub fn encoder_ref(&self) -> &E {
        &self.encoder
    }

    /// Returns a mutable reference to the encoder being used by the instance.
    pub fn encoder_mut(&mut self) -> &mut E {
        &mut self.encoder
    }
}
impl<E: Encode, D: Decode> Transport for TcpTransporter<E, D> {
    type PeerAddr = ();
    type SendItem = E::Item;
    type RecvItem = D::Item;

    fn start_send(&mut self, (): Self::PeerAddr, item: Self::SendItem) -> Result<()> {
        self.outgoing_queue.push_back(item);
        track!(self.poll_send())?;
        Ok(())
    }

    fn poll_send(&mut self) -> PollSend {
        loop {
            track!(self.stream.execute_io())?;
            track!(
                self.encoder
                    .encode_to_write_buf(self.stream.write_buf_mut())
            )?;
            if self.encoder.is_idle() {
                if let Some(item) = self.outgoing_queue.pop_front() {
                    track!(self.encoder.start_encoding(item))?;
                } else if self.stream.write_buf_ref().is_empty() {
                    return Ok(Async::Ready(()));
                }
            }
            if self.stream.would_block() || self.stream.is_eos() {
                return Ok(Async::NotReady);
            }
        }
    }

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)> {
        loop {
            track!(self.stream.execute_io())?;
            track!(
                self.decoder
                    .decode_from_read_buf(self.stream.read_buf_mut())
            )?;
            if self.decoder.is_idle() {
                let item = track!(self.decoder.finish_decoding())?;
                return Ok(Async::Ready(Some(((), item))));
            }
            if self.stream.is_eos() {
                return Ok(Async::Ready(None));
            }
            if self.stream.would_block() {
                return Ok(Async::NotReady);
            }
        }
    }
}
impl<E: Encode, D: Decode> TcpTransport for TcpTransporter<E, D> {
    fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}
