use std::error::Error;
use std::fmt::{Display, Formatter, Result};

/// Defines the basic error types that can be encountered.
#[derive(Debug, Clone)]
pub enum ServerError {
    /// An error around key access - missing key or filled key
    KeyError(String),
    /// Networking issue
    NetworkError(String),
    /// Cannot write for some reason
    WriteError(String),
    ///  Tokenization errors
    TokenizationError(String),
    /// Parsing fails
    ParseError(String),
    /// Indexing in a vector or map fails
    IndexError(String),
    /// The type of something is not what was expected
    TypeError(String),
    /// A lifetime is incorrect (0 or negative)
    InvalidLifetimeError(String),
    /// Random internal errors
    InternalError(String),
    /// Catchall for anything else
    OtherError(String),
    /// Authentication errors
    UnauthorizedError(String)
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let (err, msg) = match self {
            ServerError::KeyError(msg) => ("KeyError", msg),
            ServerError::NetworkError(msg) => ("NetworkError", msg),
            ServerError::WriteError(msg) => ("WriteError", msg),
            ServerError::TokenizationError(msg) => ("TokenizationError", msg),
            ServerError::ParseError(msg) => ("ParseError", msg),
            ServerError::IndexError(msg) => ("IndexError", msg),
            ServerError::TypeError(msg) => ("TypeError", msg),
            ServerError::InvalidLifetimeError(msg) => ("InvalidLifetimeError", msg),
            ServerError::InternalError(msg) => ("OtherError", msg),
            ServerError::OtherError(msg) => ("InternalError", msg),
            ServerError::UnauthorizedError(msg) => ("UnauthorizedError", msg),
        };
        write!(f, "{}: {}", err, msg)
    }
}

impl Error for ServerError {}
