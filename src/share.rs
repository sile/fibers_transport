use std::cell::RefCell;
use std::rc::Rc;

use base::Transport;
use {PollRecv, PollSend, Result};

#[derive(Debug)]
pub struct RcTransporter<T>(Rc<RefCell<T>>);
impl<T: Transport> RcTransporter<T> {
    pub fn new(inner: T) -> Self {
        RcTransporter(Rc::new(RefCell::new(inner)))
    }

    pub fn with_inner_ref<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        f(&self.0.borrow())
    }

    pub fn with_inner_mut<F, U>(&mut self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        f(&mut self.0.borrow_mut())
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
        self.0.borrow_mut().start_send(peer, item)
    }

    fn poll_send(&mut self) -> PollSend {
        self.0.borrow_mut().poll_send()
    }

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)> {
        self.0.borrow_mut().poll_recv()
    }
}
