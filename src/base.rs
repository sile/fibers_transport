use futures::{Async, Future, Poll};
use std::fmt::Debug;
use std::hash::Hash;

use {Error, ErrorKind, PollRecv, PollSend, Result};

/// This trait allows for sending and receiving items between peers.
pub trait Transport {
    /// Peer address.
    type PeerAddr: Eq + Clone + Hash + Debug;

    /// Outgoing item.
    type SendItem;

    /// Incoming item.
    type RecvItem;

    /// Starts sending the given item to the destination peer.
    fn start_send(&mut self, peer: Self::PeerAddr, item: Self::SendItem) -> Result<()>;

    /// Polls the transmission of the all outstanding items in the transporter have been completed.
    ///
    /// If it has been completed, this will return `Ok(Async::Ready(()))`.
    fn poll_send(&mut self) -> PollSend;

    /// Polls reception of an item from a peer.
    ///
    /// If the transporter has terminated, this will return `Ok(Async::Ready(None))`.
    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)>;
}

/// Returns a future that waits the transmission of the all outstanding items in
/// the given transporter have been completed.
pub fn wait_send<T: Transport>(transporter: T) -> impl Future<Item = T, Error = Error> {
    WaitSend(Some(transporter))
}

/// Returns a future that waits until the given transporter receives an item from a peer.
pub fn wait_recv<T: Transport>(
    transporter: T,
) -> impl Future<Item = (T, T::PeerAddr, T::RecvItem), Error = Error> {
    WaitRecv(Some(transporter))
}

#[derive(Debug)]
struct WaitSend<T>(Option<T>);
impl<T: Transport> Future for WaitSend<T> {
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        {
            let t = self.0.as_mut().expect("Cannot poll WaitSend twice");
            if !track!(t.poll_send())?.is_ready() {
                return Ok(Async::NotReady);
            }
        }
        Ok(Async::Ready(self.0.take().expect("never fails")))
    }
}

#[derive(Debug)]
struct WaitRecv<T>(Option<T>);
impl<T: Transport> Future for WaitRecv<T> {
    type Item = (T, T::PeerAddr, T::RecvItem);
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (peer, item) = {
            let t = self.0.as_mut().expect("Cannot poll WaitRecv twice");
            match track!(t.poll_recv())? {
                Async::NotReady => return Ok(Async::NotReady),
                Async::Ready(None) => {
                    track_panic!(ErrorKind::Other, "Transporter unexpectedly terminated")
                }
                Async::Ready(Some((peer, item))) => (peer, item),
            }
        };

        let transporter = self.0.take().expect("never fails");
        Ok(Async::Ready((transporter, peer, item)))
    }
}
