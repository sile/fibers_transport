//! Transport layer abstraction built on top of [`fibers`] crate.
//!
//! [`fibers`]: https://crates.io/crates/fibers
//!
//! # Examples
//!
//! UDP peers.
//!
//! ```
//! # extern crate bytecodec;
//! # extern crate fibers_global;
//! # extern crate fibers_transport;
//! # extern crate trackable;
//! use bytecodec::bytes::{Utf8Decoder, Utf8Encoder};
//! use fibers_transport::{Transport, UdpTransport, UdpTransporter, wait_send, wait_recv};
//!
//! type UdpPeer = UdpTransporter<Utf8Encoder, Utf8Decoder>;
//!
//! # fn main() -> Result<(), trackable::error::MainError> {
//! // Binds peers.
//! let mut peer0 = fibers_global::execute(UdpPeer::bind("127.0.0.1:0".parse().unwrap()))?;
//! let peer1 = fibers_global::execute(UdpPeer::bind("127.0.0.1:0".parse().unwrap()))?;
//!
//! // `peer0` sends a message to `peer1`.
//! peer0.start_send(peer1.local_addr(), "foo".to_owned())?;
//! let peer0 = fibers_global::execute(wait_send(peer0))?;
//!
//! // `peer1` receives a message from `peer0`.
//! let (_, addr, item) = fibers_global::execute(wait_recv(peer1))?;
//! assert_eq!(addr, peer0.local_addr());
//! assert_eq!(item, "foo");
//! # Ok(())
//! # }
//! ```
//!
//! TCP server and client.
//!
//! ```
//! # extern crate bytecodec;
//! # extern crate factory;
//! # extern crate fibers_global;
//! # extern crate fibers_transport;
//! # extern crate futures;
//! # extern crate trackable;
//! use bytecodec::fixnum::{U8Decoder, U8Encoder};
//! use factory::DefaultFactory;
//! use fibers_transport::{Transport, TcpListener, TcpTransport, TcpTransporter, wait_send, wait_recv};
//! use futures::Stream;
//!
//! type TcpServer = TcpListener<DefaultFactory<U8Encoder>, DefaultFactory<U8Decoder>>;
//! type TcpClient = TcpTransporter<U8Encoder, U8Decoder>;
//!
//! # fn main() -> Result<(), trackable::error::MainError> {
//! let server = fibers_global::execute(TcpServer::listen("127.0.0.1:0".parse().unwrap()))?;
//! let mut client = fibers_global::execute(TcpClient::connect(server.local_addr()))?;
//!
//! // Sends a message to the server.
//! client.start_send((), 123)?;
//! let client = fibers_global::execute(wait_send(client))?;
//!
//! // Receives the message from the client.
//! let (server, _) = fibers_global::execute(server.into_future()).map_err(|(e, _)| e)?;
//! let server = server.unwrap();
//! assert_eq!(server.peer_addr(), client.local_addr());
//!
//! let (mut server, _, item) = fibers_global::execute(wait_recv(server))?;
//! assert_eq!(item, 123);
//!
//! // Replies to the client.
//! server.start_send((), 9)?;
//! let _ = fibers_global::execute(wait_send(server))?;
//!
//! // Receives the reply from the server.
//! let (_, _, item) = fibers_global::execute(wait_recv(client))?;
//! assert_eq!(item, 9);
//! # Ok(())
//! # }
//! ```
#![warn(missing_docs)]
extern crate bytecodec;
extern crate factory;
extern crate fibers;
#[cfg(test)]
extern crate fibers_global;
extern crate futures;
#[macro_use]
extern crate trackable;

pub use base::{wait_recv, wait_send, Transport};
pub use error::{Error, ErrorKind};
pub use fixed_peer::FixedPeerTransporter;
pub use peer_addr::PeerAddr;
pub use share::RcTransporter;
pub use tcp::{TcpTransport, TcpTransporter, TcpTransporterBuilder};
pub use tcp_listener::{TcpListener, TcpListenerBuilder};
pub use udp::{UdpTransport, UdpTransporter, UdpTransporterBuilder};

mod base;
mod error;
mod fixed_peer;
mod peer_addr;
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

#[cfg(test)]
mod tests {
    use bytecodec::bytes::{Utf8Decoder, Utf8Encoder};
    use bytecodec::fixnum::{U8Decoder, U8Encoder};
    use factory::DefaultFactory;
    use futures::Stream;
    use std::result::Result;
    use trackable;

    use super::*;

    #[test]
    fn basic_udp_test() -> Result<(), trackable::error::MainError> {
        type Udp = UdpTransporter<Utf8Encoder, Utf8Decoder>;

        let mut peer0 = fibers_global::execute(Udp::bind("127.0.0.1:0".parse().unwrap()))?;
        let peer1 = fibers_global::execute(Udp::bind("127.0.0.1:0".parse().unwrap()))?;

        peer0.start_send(peer1.local_addr(), "foo".to_owned())?;
        let peer0 = fibers_global::execute(wait_send(peer0))?;

        let (_, addr, item) = fibers_global::execute(wait_recv(peer1))?;
        assert_eq!(addr, peer0.local_addr());
        assert_eq!(item, "foo");

        Ok(())
    }

    #[test]
    fn basic_tcp_test() -> Result<(), trackable::error::MainError> {
        type TcpServer = TcpListener<DefaultFactory<U8Encoder>, DefaultFactory<U8Decoder>>;
        type TcpClient = TcpTransporter<U8Encoder, U8Decoder>;

        let server = fibers_global::execute(TcpServer::listen("127.0.0.1:0".parse().unwrap()))?;
        let mut client = fibers_global::execute(TcpClient::connect(server.local_addr()))?;

        client.start_send((), 123)?;
        let client = fibers_global::execute(wait_send(client))?;

        let (server, _) = fibers_global::execute(server.into_future()).map_err(|(e, _)| e)?;
        let server = server.unwrap();
        assert_eq!(server.peer_addr(), client.local_addr());

        let (mut server, _, item) = fibers_global::execute(wait_recv(server))?;
        assert_eq!(item, 123);

        server.start_send((), 9)?;
        let _ = fibers_global::execute(wait_send(server))?;

        let (_, _, item) = fibers_global::execute(wait_recv(client))?;
        assert_eq!(item, 9);

        Ok(())
    }
}
