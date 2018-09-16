use bytecodec::{Decode, DecodeExt, Encode, EncodeExt};
use fibers::net::futures::{RecvFrom, SendTo};
use fibers::net::UdpSocket;
use futures::Poll;
use futures::{Async, Future};
use std::collections::VecDeque;
use std::net::SocketAddr;

use base::Transport;
use {Error, ErrorKind, PollRecv, PollSend, Result};

/// This trait indicates that the implementation implements UDP.
pub trait UdpTransport: Transport<PeerAddr = SocketAddr> {}

/// [`UdpTransporter`] builder.
///
/// [`UdpTransporter`]: ./struct.UdpTransporter.html
#[derive(Debug, Clone)]
pub struct UdpTransporterBuilder<E, D> {
    buf_size: usize,
    encoder: E,
    decoder: D,
}
impl<E, D> UdpTransporterBuilder<E, D>
where
    E: Encode + Default,
    D: Decode + Default,
{
    /// Makes a new `UdpTransporterBuilder` instance with the default settings.
    pub fn new() -> Self {
        Self::default()
    }
}
impl<E: Encode, D: Decode> UdpTransporterBuilder<E, D> {
    /// Makes a new `UdpTransporterBuilder` instance with the given encoder and decoder.
    pub fn with_codec(encoder: E, decoder: D) -> Self {
        UdpTransporterBuilder {
            buf_size: 4096,
            encoder,
            decoder,
        }
    }

    /// Sets the size of the send and receive buffer of the resulting instance in byte.
    ///
    /// The default value is `4096`.
    pub fn buf_size(mut self, size: usize) -> Self {
        self.buf_size = size;
        self
    }

    /// Makes a new `UdpTransporter` instance with the given settings.
    pub fn finish(self, socket: UdpSocket) -> UdpTransporter<E, D> {
        let recv_from = socket.clone().recv_from(vec![0; self.buf_size]);
        UdpTransporter {
            socket,
            encoder: self.encoder,
            decoder: self.decoder,
            outgoing_queue: VecDeque::new(),
            send_to: None,
            recv_from,
        }
    }

    /// Starts binding to the specified address and will makes
    /// a new `UdpTransporter` instance if the operation is succeeded.
    pub fn bind(self, addr: SocketAddr) -> impl Future<Item = UdpTransporter<E, D>, Error = Error> {
        UdpSocket::bind(addr)
            .map(move |socket| self.finish(socket))
            .map_err(|e| track!(Error::from(e)))
    }
}
impl<E, D> Default for UdpTransporterBuilder<E, D>
where
    E: Encode + Default,
    D: Decode + Default,
{
    fn default() -> Self {
        Self::with_codec(E::default(), D::default())
    }
}

/// An implementation of [`Transport`] that uses UDP as the transport layer.
///
/// [`Transport`]: ./trait.Transport.html
#[derive(Debug)]
pub struct UdpTransporter<E: Encode, D: Decode> {
    socket: UdpSocket,
    encoder: E,
    decoder: D,
    outgoing_queue: VecDeque<(SocketAddr, E::Item)>,
    send_to: Option<SendTo<Vec<u8>>>,
    recv_from: RecvFrom<Vec<u8>>,
}
impl<E, D> UdpTransporter<E, D>
where
    E: Encode + Default,
    D: Decode + Default,
{
    /// Starts binding to the specified address and will makes
    /// a new `UdpTransporter` instance if the operation is succeeded.
    ///
    /// This is equivalent to `UdpTransporterBuilder::default().bind(addr)`.
    pub fn bind(addr: SocketAddr) -> impl Future<Item = Self, Error = Error> {
        UdpTransporterBuilder::default().bind(addr)
    }
}
impl<E: Encode, D: Decode> UdpTransporter<E, D> {
    /// Returns the number of unsent messages in the queue of the instance.
    pub fn message_queue_len(&self) -> usize {
        self.outgoing_queue.len() + if self.encoder.is_idle() { 0 } else { 1 }
    }

    /// Returns a reference to the UDP socket being used by the instance.
    pub fn socket_ref(&self) -> &UdpSocket {
        &self.socket
    }

    /// Returns a mutable reference to the UDP socket being used by the instance.
    pub fn socket_mut(&mut self) -> &mut UdpSocket {
        &mut self.socket
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

    fn poll_send_to(&mut self) -> Poll<(), Error> {
        match self.send_to.poll() {
            Err((_, _, e)) => Err(track!(Error::from(e))),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(())),
            Ok(Async::Ready(Some((_, buf, written_size)))) => {
                track_assert_eq!(buf.len(), written_size, ErrorKind::Other);
                self.send_to = None;
                Ok(Async::Ready(()))
            }
        }
    }
}
impl<E, D> From<UdpSocket> for UdpTransporter<E, D>
where
    E: Encode + Default,
    D: Decode + Default,
{
    fn from(f: UdpSocket) -> Self {
        UdpTransporterBuilder::default().finish(f)
    }
}
impl<E: Encode, D: Decode> Transport for UdpTransporter<E, D> {
    type PeerAddr = SocketAddr;
    type SendItem = E::Item;
    type RecvItem = D::Item;

    fn start_send(&mut self, peer: Self::PeerAddr, item: E::Item) -> Result<()> {
        self.outgoing_queue.push_back((peer, item));
        track!(self.poll_send())?;
        Ok(())
    }

    fn poll_send(&mut self) -> PollSend {
        while track!(self.poll_send_to())?.is_ready() {
            if let Some((peer, item)) = self.outgoing_queue.pop_front() {
                // FIXME: optimize
                let bytes = track!(self.encoder.encode_into_bytes(item))?;
                self.send_to = Some(self.socket.clone().send_to(bytes, peer));
            } else {
                return Ok(Async::Ready(()));
            }
        }
        Ok(Async::NotReady)
    }

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, D::Item)> {
        if let Async::Ready((socket, buf, size, peer)) = self
            .recv_from
            .poll()
            .map_err(|(_, _, e)| track!(Error::from(e)))?
        {
            let item = track!(self.decoder.decode_from_bytes(&buf[..size]))?;
            self.recv_from = socket.recv_from(buf);
            Ok(Async::Ready(Some((peer, item))))
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl<E: Encode, D: Decode> UdpTransport for UdpTransporter<E, D> {}
