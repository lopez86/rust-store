/// Defines the basic error types that can be encountered.
pub enum ServerError {
    /// An error around key access - missing key or filled key
    KeyError(String),
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
    
    /// Catchall for anything else
    OtherError(String),
}
