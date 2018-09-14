use std::net::SocketAddr;

use base::Transport;

pub trait UdpTransport: Transport<Address = SocketAddr> {}
