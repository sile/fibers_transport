//! Transport layer abstraction built on top of [`fibers`] crate.
//!
//! [`fibers`]: https://crates.io/crates/fibers
#![warn(missing_docs)]
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

/// This crate specific [`Result`] type.
///
/// [`Result`]: https://doc.rust-lang.org/std/result/enum.Result.html
pub type Result<T> = std::result::Result<T, Error>;

/// The return type of the [`Transport::poll_send`] method.
///
/// [`Transport::poll_send`]: ./trait.Transport.html#tymethod.poll_send
pub type PollSend = futures::Poll<(), Error>;

/// The return type of the [`Transport::poll_recv`] method.
///
/// [`Transport::poll_recv`]: ./trait.Transport.html#tymethod.poll_recv
pub type PollRecv<T> = futures::Poll<Option<T>, Error>;
