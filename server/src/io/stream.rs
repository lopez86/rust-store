use std::collections::HashMap;

use crate::analysis::InterpreterResponse;
use crate::error::ServerError;


/// A raw request to send to the analysis worker
pub struct StreamRequest {
    /// The string of the actual command to be run
    pub request: Result<String, ServerError>,
    /// The html headers for this request
    pub headers: HashMap<String, String>,
    /// The handler to send a response back
    pub sender: Option<Box<dyn StreamSender + Send>>,
}

/// An object to hold stream information to return responses
pub trait StreamSender {
    /// Send a response back out
    fn send(&mut self, response: Result<InterpreterResponse, ServerError>) -> Result<(), ServerError>;
}


/// An object to translate an incoming connection and return requests
pub trait StreamHandler {

    /// Receive a request
    fn receive_request(&mut self) -> Option<StreamRequest>;
}
