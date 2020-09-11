use futures::Async;
use std::net::SocketAddr;

use {ErrorKind, PeerAddr, PollRecv, PollSend, Result, TcpTransport, Transport, UdpTransport};

/// An implementation of [`Transport`] used for communicating with a fixed peer.
///
/// [`Transport`]: ./trait.Transport.html
#[derive(Debug)]
pub struct FixedPeerTransporter<T: Transport, P = <T as Transport>::PeerAddr> {
    exterior_peer: P,
    interior_peer: T::PeerAddr,
    inner: T,
}
impl<T: Transport, P> FixedPeerTransporter<T, P> {
    /// Makes a new `FixedPeerTransporter` instance.
    pub fn new(exterior_peer: P, interior_peer: T::PeerAddr, inner: T) -> Self {
        FixedPeerTransporter {
            exterior_peer,
            interior_peer,
            inner,
        }
    }

    /// Returns a reference to the inner transporter.
    pub fn inner_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the inner transporter.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Returns a reference to the fixed peer address exposed to users of the transporter.
    pub fn exterior_peer(&self) -> &P {
        &self.exterior_peer
    }

    /// Returns a reference to the fixed peer address used internally in the transporter.
    pub fn interior_peer(&self) -> &T::PeerAddr {
        &self.interior_peer
    }
}
impl<T: Transport, P: PeerAddr> Transport for FixedPeerTransporter<T, P> {
    type PeerAddr = P;
    type SendItem = T::SendItem;
    type RecvItem = T::RecvItem;

    fn start_send(&mut self, peer: Self::PeerAddr, item: Self::SendItem) -> Result<()> {
        track_assert_eq!(
            peer,
            self.exterior_peer,
            ErrorKind::InvalidInput,
            "Unexpected destination peer"
        );
        self.inner.start_send(self.interior_peer.clone(), item)
    }

    fn poll_send(&mut self) -> PollSend {
        self.inner.poll_send()
    }

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)> {
        loop {
            match self.inner.poll_recv()? {
                Async::NotReady => return Ok(Async::NotReady),
                Async::Ready(None) => return Ok(Async::Ready(None)),
                Async::Ready(Some((peer, item))) => {
                    if peer == self.interior_peer {
                        return Ok(Async::Ready(Some((self.exterior_peer.clone(), item))));
                    }
                }
            }
        }
    }
}
impl<T: UdpTransport> UdpTransport for FixedPeerTransporter<T, SocketAddr> {
    fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }
}
impl<T: TcpTransport> From<T> for FixedPeerTransporter<T, SocketAddr> {
    fn from(f: T) -> Self {
        FixedPeerTransporter::new(f.peer_addr(), (), f)
    }
}
