use bytecodec::io::{BufferedIo, IoDecodeExt, IoEncodeExt};
use bytecodec::{Decode, Encode};
use fibers::net::TcpStream;
use futures::{Async, Future};
use std::collections::VecDeque;
use std::net::SocketAddr;

use base::Transport;
use {Error, PollRecv, PollSend, Result};

pub trait TcpTransport: Transport<PeerAddr = ()> {}

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
    pub fn new() -> Self {
        Self::default()
    }
}
impl<E: Encode, D: Decode> TcpTransporterBuilder<E, D> {
    pub fn with_codec(encoder: E, decoder: D) -> Self {
        TcpTransporterBuilder {
            buf_size: 8192,
            encoder,
            decoder,
        }
    }

    pub fn buf_size(mut self, size: usize) -> Self {
        self.buf_size = size;
        self
    }

    pub fn connect(
        self,
        peer: SocketAddr,
    ) -> impl Future<Item = TcpTransporter<E, D>, Error = Error> {
        TcpStream::connect(peer)
            .map(move |stream| self.finish(stream))
            .map_err(|e| track!(Error::from(e)))
    }

    pub fn finish(self, stream: TcpStream) -> TcpTransporter<E, D> {
        let _ = stream.set_nodelay(true);
        TcpTransporter {
            stream: BufferedIo::new(stream, self.buf_size, self.buf_size),
            encoder: self.encoder,
            decoder: self.decoder,
            outgoing_queue: VecDeque::new(),
        }
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
    pub fn connect(peer: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        TcpTransporterBuilder::new().connect(peer)
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
impl<E, D> From<TcpStream> for TcpTransporter<E, D>
where
    E: Encode + Default,
    D: Decode + Default,
{
    fn from(stream: TcpStream) -> Self {
        TcpTransporterBuilder::new().finish(stream)
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
impl<E: Encode, D: Decode> TcpTransport for TcpTransporter<E, D> {}
