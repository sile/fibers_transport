use {PollRecv, PollSend, Result};

pub trait Transport {
    type PeerAddr;
    type SendItem;
    type RecvItem;

    fn start_send(&mut self, peer: Self::PeerAddr, item: Self::SendItem) -> Result<()>;

    fn poll_send(&mut self) -> PollSend;

    fn poll_recv(&mut self) -> PollRecv<(Self::PeerAddr, Self::RecvItem)>;
}
