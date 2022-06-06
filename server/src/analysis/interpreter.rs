use crate::analysis::Statement;
use crate::error::ServerError;
use crate::storage::{Storage, StorageKey, StorageValue};

/// Defines the different privilege levels that can be attached to a request.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Privileges {
    /// Admins can do anything
    Admin,
    /// Allows read-only operations
    Read,
    /// Allows read and write operations
    Write,
    /// Allows nothing - always returns an error
    Unauthorized,
}

/// A request to the interpreter
pub struct InterpreterRequest {
    /// The statement to be processed
    pub statement: Statement,
    /// Privileges available to this request
    pub privileges: Privileges,
}

/// A response from the interpreter
#[derive(Clone, Debug)]
pub enum InterpreterResponse {
    /// Get a value from the key value store
    Value(StorageValue),
    /// A string message response
    Message(String),
    /// Get the size of something
    Size(usize),
    /// Get a key
    Key(StorageKey),
    /// Get a boolean value
    Bool(bool),
}

/// An interpreter backed by some storage 
pub struct Interpreter<S: Storage> {
    /// The underlying storage to communicate with
    pub storage: S,
}

impl<S: Storage> Interpreter<S> {
    /// Create a new interpreter for the storage
    pub fn new(storage: S) -> Interpreter<S> {
        Interpreter{storage}
    }

    /// Interpret a request
    pub fn interpret(&mut self, _request: InterpreterRequest) -> Result<InterpreterResponse, ServerError> {
        Err(ServerError::OtherError("Not implemented yet.".to_string()))
    }
}
