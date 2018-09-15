use base::Transport;

pub trait TcpTransport: Transport<PeerAddr = ()> {}
