use serde::{Deserialize, Serialize};

use crate::analysis::Statement;
use crate::auth::AuthorizationLevel;
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
    pub statements: Vec<Statement>,
    /// Privileges available to this request
    pub authorization: AuthorizationLevel,
}

/// A response from the interpreter
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    /// Shutting down the server
    ShuttingDown,
    /// No response
    Null,
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
        Err(ServerError::InternalError("Not implemented yet.".to_string()))
    }

    
    fn process_statements(
        &mut self, statements: &Vec<Statement>, authorization: AuthorizationLevel
    ) -> Result<InterpreterResponse, ServerError> {
        validate_authorization(statements, authorization)?;
        let mut final_response: Result<InterpreterResponse, ServerError>;
        let mut keep_going = true;
        for statement in statements {
            final_response = self.process_statement(statement);
            if let Ok(InterpreterResponse::ShuttingDown) = final_response {
                break;
            }
            if let Err(error) = final_response {
                break;
            }
        }
        final_response
    }

    fn process_statement(
        &mut self, statement: &Statement
    ) -> Result<InterpreterResponse, ServerError> {
        if let Statement::Shutdown = statement {
            return Ok(InterpreterResponse::ShuttingDown);
        }
        Err(ServerError::InternalError("Interpreter not yet implemented for most cases.".to_string()))
    }

}


fn validate_authorization(
    statements: &Vec<Statement>, authorization: AuthorizationLevel
) -> Result<(), ServerError> {
    let is_authorized = true;
    for statement in statements.iter() {
        is_authorized = match statement {
            Statement::Shutdown => authorization == AuthorizationLevel::Admin,
            Statement::Delete(..) | Statement::Set(..) | Statement::SetIfNotExists(..) |
            Statement::VectorSet(..) | Statement::VectorAppend(..) | Statement::VectorPop(..) |
            Statement::MapSet(..) | Statement::MapDelete(..) | Statement::Update(..) |
            Statement::UpdateLifetime(..) => (authorization == AuthorizationLevel::Admin) |
                (authorization == AuthorizationLevel::Write),
            _ => true,
        };
        if !is_authorized {
            break;
        }
    }

    if is_authorized {
        Ok(())
    } else {
        Err(ServerError::AuthorizationError("User is not authorized to perform this query.".to_string()))
    }
}
