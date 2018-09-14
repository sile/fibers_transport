use std::net::SocketAddr;

use base::Transport;

pub trait TcpTransport: Transport<Address = SocketAddr> {}
