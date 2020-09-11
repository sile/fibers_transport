use crate::base::Transport;
use crate::{PollRecv, PollSend, Result, TcpTransport, UdpTransport};
use futures::Async;
use std::cell::RefCell;
use std::fmt;
use std::net::SocketAddr;
use std::rc::Rc;

/// Shareable transporter.
#[derive(Debug)]
pub struct RcTransporter<T: Transport>(Rc<RefCell<Inner<T>>>);
impl<T: Transport> RcTransporter<T> {
    /// Makes a new `RcTransporter` instance.
    pub fn new(inner: T) -> Self {
        let inner = Inner {
            transporter: inner,
            peek_recv: None,
        };
        RcTransporter(Rc::new(RefCell::new(inner)))
    }

    /// Executes the given function with a reference to the inner transporter.
    pub fn with_inner_ref<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        f(&self.0.borrow().transporter)
    }

    /// Executes the given function with a mutable reference to the inner transporter.
    pub fn with_inner_mut<F, U>(&mut self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        f(&mut self.0.borrow_mut().transporter)
    }

    /// Executes the given function with the next incoming item if it is available.
    ///
    /// If there is no such item, this will return `Ok(None)` without executing `f`.
    pub fn with_peek_recv<F, U>(&mut self, f: F) -> Result<Option<U>>
    where
        F: FnOnce(&T::PeerAddr, &T::RecvItem) -> U,
    {
        let mut inner = self.0.borrow_mut();
        if inner.peek_recv.is_some() {
            Ok(inner.peek_recv.as_ref().map(|x| f(&x.0, &x.1)))
        } else if let Async::Ready(Some((peer, item))) = track!(inner.transporter.poll_recv())? {
            inner.peek_recv = Some((peer, item));
            Ok(inner.peek_recv.as_ref().map(|x| f(&x.0, &x.1)))
        } else {
            Ok(None)
        }
    }
}
impl<T: Transport> Clone for RcTransporter<T> {
    fn clone(&self) -> Self {
        RcTransporter(self.0.clone())
    }
}
impl<T: Transport> Transport for RcTransporter<T> {
    type PeerAddr = T::PeerAddr;
    type SendItem = T::SendItem;
    type RecvItem = T::RecvItem;

    fn start_send(&mut self, peer: Self::PeerAddr, item: Self::SendItem) -> Result<()> {
        track!(self.0.borrow_mut().transporter.start_send(peer, item))
    }

    fn poll_send(&mut self) -> PollSend {
        track!(self.0.borrow_mut().transporter.poll_send())
    }

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)> {
        let mut inner = self.0.borrow_mut();
        if let Some((peer, item)) = inner.peek_recv.take() {
            Ok(Async::Ready(Some((peer, item))))
        } else {
            track!(inner.transporter.poll_recv())
        }
    }
}
impl<T: TcpTransport> TcpTransport for RcTransporter<T> {
    fn peer_addr(&self) -> SocketAddr {
        self.0.borrow().transporter.peer_addr()
    }

    fn local_addr(&self) -> SocketAddr {
        self.0.borrow().transporter.local_addr()
    }
}
impl<T: UdpTransport> UdpTransport for RcTransporter<T> {
    fn local_addr(&self) -> SocketAddr {
        self.0.borrow().transporter.local_addr()
    }
}

struct Inner<T: Transport> {
    transporter: T,
    peek_recv: Option<(T::PeerAddr, T::RecvItem)>,
}
impl<T> fmt::Debug for Inner<T>
where
    T: Transport + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.transporter)
    }
}
