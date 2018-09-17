use std::fmt::Debug;
use std::hash::Hash;
use std::net::SocketAddr;

/// Peer address.
pub trait PeerAddr: Clone + Eq + Hash + Debug {}
impl PeerAddr for () {}
impl PeerAddr for SocketAddr {}
