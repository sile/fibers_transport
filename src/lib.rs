extern crate bytecodec;
extern crate factory;
extern crate fibers;
extern crate futures;
#[macro_use]
extern crate trackable;

pub use base::Transport;
pub use error::{Error, ErrorKind};
pub use share::RcTransporter;
pub use tcp::{TcpTransport, TcpTransporter, TcpTransporterBuilder};
pub use tcp_listener::{TcpListener, TcpListenerBuilder};
pub use udp::{UdpTransport, UdpTransporter, UdpTransporterBuilder};

mod base;
mod error;
mod share;
mod tcp;
mod tcp_listener;
mod udp;

pub type Result<T> = std::result::Result<T, Error>;
pub type PollSend = futures::Poll<(), Error>;
pub type PollRecv<T> = futures::Poll<Option<T>, Error>;
