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
    /// Tokenization errors
    TokenizationError(String),
    /// Parsing fails
    ParseError(String),
    /// Indexing in a vector or map fails
    IndexError(String),
    /// The type of something is not what was expected
    TypeError(String),
    /// Catchall for anything else
    InternalError(String),
    /// Authorization errors
    AuthorizationError(String),
    /// Authentication error
    AuthenticationError(String),
    /// Error 
    RequestError(String),
}

pub fn get_error_code(error: &ServerError) -> String {
    let err_string = match error {
        ServerError::KeyError(_) => "422 Unprocessible Entity",
        ServerError::NetworkError(_) => "500 Internal Service Error",
        ServerError::WriteError(_) => "500 Internal Service Error",
        ServerError::TokenizationError(_) => "422 Unprocessible Entity",
        ServerError::ParseError(_) => "422 Unprocessible Entity",
        ServerError::IndexError(_) => "422 Unprocessible Entity",
        ServerError::TypeError(_) => "422 Unprocessible Entity",
        ServerError::InternalError(_) => "500 Internal Service Error",
        ServerError::AuthorizationError(_) => "401 Unauthorized",
        ServerError::AuthenticationError(_) => "403 Forbidden",
        ServerError::RequestError(_) => "400 Bad Request",
    };
    err_string.to_string()
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
            ServerError::InternalError(msg) => ("InternalError", msg),
            ServerError::AuthorizationError(msg) => ("AuthorizationError", msg),
            ServerError::AuthenticationError(msg) => ("AuthenticationError", msg),
            ServerError::RequestError(msg) => ("RequestError", msg),
        };
        write!(f, "{}: {}", err, msg)
    }
}

impl Error for ServerError {}
