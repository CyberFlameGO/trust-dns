// Copyright 2015-2020 Benjamin Fry <benjaminfry@me.com>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Error types for the crate

use std::{fmt, io};

use futures_channel::mpsc;
use thiserror::Error;
use trust_dns_proto::error::{ProtoError, ProtoErrorKind};

use crate::error::{DnsSecError, DnsSecErrorKind};
use crate::proto::{trace, ExtBacktrace};

/// An alias for results returned by functions of this crate
pub type Result<T> = ::std::result::Result<T, Error>;

/// The error kind for errors that get returned in the crate
#[derive(Debug, Error)]
pub enum ErrorKind {
    /// An error with an arbitrary message, referenced as &'static str
    #[error("{0}")]
    Message(&'static str),

    /// An error with an arbitrary message, stored as String
    #[error("{0}")]
    Msg(String),

    // foreign
    /// A dnssec error
    #[error("dnssec error")]
    DnsSec(#[from] DnsSecError),

    /// An error got returned from IO
    #[error("io error")]
    Io(#[from] std::io::Error),

    /// An error got returned by the trust-dns-proto crate
    #[error("proto error")]
    Proto(#[from] ProtoError),

    /// Queue send error
    #[error("error sending to mpsc: {0}")]
    SendError(#[from] mpsc::SendError),

    /// A request timed out
    #[error("request timed out")]
    Timeout,
}

impl Clone for ErrorKind {
    fn clone(&self) -> Self {
        use self::ErrorKind::*;
        match self {
            Message(msg) => Message(msg),
            Msg(ref msg) => Msg(msg.clone()),
            // foreign
            DnsSec(dnssec) => DnsSec(dnssec.clone()),
            Io(io) => Io(std::io::Error::from(io.kind())),
            Proto(proto) => Proto(proto.clone()),
            SendError(e) => SendError(e.clone()),
            Timeout => Timeout,
        }
    }
}

/// The error type for errors that get returned in the crate
#[derive(Debug, Error, Clone)]
pub struct Error {
    kind: ErrorKind,
    backtrack: Option<ExtBacktrace>,
}

impl Error {
    /// Get the kind of the error
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref backtrace) = self.backtrack {
            fmt::Display::fmt(&self.kind, f)?;
            fmt::Debug::fmt(backtrace, f)
        } else {
            fmt::Display::fmt(&self.kind, f)
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            kind,
            backtrack: trace!(),
        }
    }
}

impl From<&'static str> for Error {
    fn from(msg: &'static str) -> Error {
        ErrorKind::Message(msg).into()
    }
}

impl From<mpsc::SendError> for Error {
    fn from(e: mpsc::SendError) -> Self {
        ErrorKind::from(e).into()
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Error {
        ErrorKind::Msg(msg).into()
    }
}

impl From<DnsSecError> for Error {
    fn from(e: DnsSecError) -> Error {
        match *e.kind() {
            DnsSecErrorKind::Timeout => ErrorKind::Timeout.into(),
            _ => ErrorKind::from(e).into(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::TimedOut => ErrorKind::Timeout.into(),
            _ => ErrorKind::from(e).into(),
        }
    }
}

impl From<ProtoError> for Error {
    fn from(e: ProtoError) -> Error {
        match *e.kind() {
            ProtoErrorKind::Timeout => ErrorKind::Timeout.into(),
            _ => ErrorKind::from(e).into(),
        }
    }
}

impl From<Error> for io::Error {
    fn from(e: Error) -> Self {
        match *e.kind() {
            ErrorKind::Timeout => io::Error::new(io::ErrorKind::TimedOut, e),
            _ => io::Error::new(io::ErrorKind::Other, e),
        }
    }
}

#[test]
fn test_conversion() {
    let io_error = io::Error::new(io::ErrorKind::TimedOut, "mock timeout");

    let error = Error::from(io_error);

    match *error.kind() {
        ErrorKind::Timeout => (),
        _ => panic!("incorrect type: {}", error),
    }
}
