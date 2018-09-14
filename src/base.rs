use {Poll, Result};

pub trait Transport {
    type Address;
    type SendItem;
    type RecvItem;

    fn start_send(&mut self, peer: Self::Address, item: Self::SendItem) -> Result<()>;

    fn poll_send(&mut self) -> Poll<()>;

    fn poll_recv(&mut self) -> Poll<Option<(Self::Address, Self::RecvItem)>>;
}
