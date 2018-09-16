use {PollRecv, PollSend, Result};

/// This trait allows for sending and receiving items between peers.
pub trait Transport {
    /// Peer address.
    type PeerAddr;

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
