/// Defines the basic error types that can be encountered.
pub enum ServerError {
    /// An error around key access - missing key or filled key
    KeyError(String),
    NetworkError(String),
    /// Cannot write for some reason
    WriteError(String),
    /// Syntax is incorrect
    SyntaxError(String),
    /// Catchall for anything else
    OtherError(String),
}
