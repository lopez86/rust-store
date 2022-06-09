use std::collections::HashMap;

use crate::error::ServerError;


#[derive(Clone, PartialEq, Debug)]
pub enum AuthenticationResult {
    /// Authentication passed, return a user id and authorization level
    Authenticated(String, Option<AuthorizationLevel>),
    /// Authentication failed - credentials rejected
    Unauthenticated,
}

pub trait AuthenticationService {
    /// Try to authenticate a request using the request headers
    fn authenticate(&mut self, headers: &HashMap<String, String>) -> Result<AuthenticationResult, ServerError>;
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AuthorizationLevel {
    /// Administrator level - can run anything
    Admin,
    /// Read/Write access - can't do administrative tasks like shutdowns
    Write,
    /// Read only access
    Read,
}

/// A simple authenticator that just looks for a username field and authenticates based on that.
/// 
/// The available usenames are:
/// - "unauthenticated" --> generates an authentication error
/// - "admin" --> user with admin privileges
/// - "write" --> user with read/write privileges
/// - "read" --> user with read-only privileges
/// - other name --> unauthorized user
/// - not present --> generates an internal error
pub struct MockAuthenticator;

impl AuthenticationService for MockAuthenticator {
    fn authenticate(&mut self, headers: &HashMap<String, String>) -> Result<AuthenticationResult, ServerError> {
        let username = match headers.get("Username") {
            Some(username) => username,
            None => {
                return Err(ServerError::InternalError("Authentication service error".to_string()))
            },
        };
        match &username[..] {
            "unauthenticated" => Ok(AuthenticationResult::Unauthenticated),
            "admin" => Ok(AuthenticationResult::Authenticated("admin".to_string(), Some(AuthorizationLevel::Admin))),
            "write" => Ok(AuthenticationResult::Authenticated("write".to_string(), Some(AuthorizationLevel::Write))),
            "read" => Ok(AuthenticationResult::Authenticated("read".to_string(), Some(AuthorizationLevel::Read))),
            username => Ok(AuthenticationResult::Authenticated(username.to_string(), None)),
        }
    }
}
