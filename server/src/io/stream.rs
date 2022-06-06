use crate::analysis::{InterpreterResponse, Privileges};
use crate::error::ServerError;


/// A raw request to send to the analysis worker
pub struct StreamRequest {
    /// The string of the actual command to be run
    pub request: Result<String, ServerError>,
    /// The privileges associated with this request
    pub privileges: Privileges,
    ///
    pub sender: Box<dyn StreamSender>,
}

/// An object to hold stream information to return responses
pub trait StreamSender {
    /// Send a response back out
    fn send(&mut self, response: InterpreterResponse) -> Result<(), ServerError>;
}

/// An object to translate an incoming connection and return requests
pub trait StreamHandler {
    /// Receive a request
    fn recv(&mut self) -> Result<StreamRequest, ServerError>;
}