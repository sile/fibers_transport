extern crate fibers;
extern crate futures;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};

mod error;

pub mod base;
pub mod tcp;
pub mod udp;

pub type Result<T> = std::result::Result<T, Error>;
pub type Poll<T> = futures::Poll<T, Error>;
