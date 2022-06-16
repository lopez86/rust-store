use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::analysis::Statement;
use crate::auth::AuthorizationLevel;
use crate::error::ServerError;
use crate::storage::{
    CollectionType,
    KeyType,
    Storage,
    StorageElement,
    StorageKey,
    StorageMap,
    StorageValue,
    StorageVector,
};

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

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ValueType {
    Null,
    Bool,
    Int,
    Float,
    String,
    Vector(CollectionType),
    Map(KeyType, CollectionType),
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
    /// When something expires
    Expiration(Option<u64>),
    /// Get a key
    Key(StorageKey),
    /// Get a boolean value
    Bool(bool),
    /// Value types
    ValueType(ValueType),
    /// Shutting down the server
    ShuttingDown,
    /// No response
    Null,
}

/// An interpreter backed by some storage 
pub struct Interpreter<S: Storage + Send> {
    /// The underlying storage to communicate with
    pub storage: S,
}

impl<S: Storage + Send> Interpreter<S> {
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
        match statement {
            Statement::Shutdown => return Ok(InterpreterResponse::ShuttingDown),
            Statement::Null => return Ok(InterpreterResponse::Null),
            Statement::Get(key) => return self.get(key),
            Statement::Exists(key) => return self.exists(key),
            Statement::GetIfExists(key) => return self.get_if_exists(key),
            Statement::GetLifetime(key) => return self.get_lifetime(key),
            Statement::ExpireKeys => return self.expire_keys(),
            Statement::Delete(key) => return self.delete(key),
            Statement::Set(key, value, lifetime) => return self.set(key, value, lifetime),
            Statement::SetIfNotExists(key, value, lifetime) => return self.set(key, value, lifetime),
            Statement::Update(key, value, lifetime) => return self.update(key, value, lifetime),
            Statement::UpdateLifetime(key, lifetime) => return self.update_lifetime(key, lifetime),
            Statement::VectorGet(key, index) => return self.vector_get(key, index),
            Statement::VectorLength(key) => return self.vector_length(key),
            Statement::VectorAppend(key, value) => return self.vector_append(key, value),
            Statement::VectorPop(key) => return self.vector_pop(key),
            Statement::VectorSet(key, index, value) => return self.vector_set(key, index, value),
            Statement::MapGet(key, element_key) => return self.map_get(key, element_key),
            Statement::MapExists(key, element_key) => return self.map_exists(key, element_key),
            Statement::MapLength(key) => return self.map_length(key),
            Statement::MapDelete(key, element_key) => return self.map_delete(key, element_key),
            Statement::MapSet(key, element_key, value) => return self.map_set(key, element_key, value),
            Statement::ValueType(key) => return self.value_type(key),
        }
    }

    fn get(&self, key: &StorageKey) -> Result<InterpreterResponse, ServerError> {
        let result = self.storage.get(key)?;
        Ok(InterpreterResponse::Value(result))
    }

    fn value_type(&self, key: &StorageKey) -> Result<InterpreterResponse, ServerError> {
        let result = self.storage.get(key)?;
        let result = match result.value {
            StorageValue::Null => ValueType::Null,
            StorageValue::Bool(_) => ValueType::Bool,
            StorageValue::Int(_) => ValueType::Int,
            StorageValue::Float(_) => ValueType::Float,
            StorageValue::String(_) => ValueType::String,
            StorageValue::Vector(v) => {
                ValueType::Vector(v.collection_type)
            },
            StorageValue::Map(m) => {
                ValueType::Map(m.key_type, m.collection_type)
            },
        };
        Ok(InterpreterResponse::ValueType(result))
    }

    fn exists(&self, key: &StorageKey) -> Result<InterpreterResponse, ServerError> {
        let result = self.storage.contains_key(key)?;
        Ok(InterpreterResponse::Bool(result))
    }

    fn get_if_exists(&self, key: &StorageKey) -> Result<InterpreterResponse, ServerError> {
        let result = self.storage.get(key)?;
        Ok(InterpreterResponse::Value(result))
    }

    fn get_lifetime(&self, key: &StorageKey) -> Result<InterpreterResponse, ServerError> {
        let current_time = SystemTime::now();
        let result = self.storage.get(key)?;
        let result = match result.expiration {
            None => None,
            Some(timestamp) => {
                let difference = timestamp.duration_since(current_time);
                match difference {
                    Err(_) => return ServerError::IndexError(format!("No entry found for key {}", key)),
                    Some(diff) => diff.as_secs(),
                }
            },
        };
        Ok(InterpreterResponse::Expiration(result))
    }

    fn expire_keys(&mut self) -> Result<InterpreterResponse, ServerError> {
        let result = self.storage.invalidate_expired_keys()?;
        Ok(InterpreterResponse::Size(result))
    }

    fn delete(&mut self, key: &StorageKey) -> Result<InterpreterResponse, ServerError> {
        let result = self.storage.delete(key)?;
        Ok(InterpreterResponse::Bool(result))
    }

    fn set(
        &mut self, key: &StorageKey, value: &StorageValue, expiration: Option<u64>
    ) -> Result<InterpreterResponse, ServerError> {
        let element = StorageElement{key, value, expiration};
        let result = self.storage.set(key, element)?;
        Ok(InterpreterResponse::Message("Ok".to_string()))
    }

    fn set_if_not_exists(
        &mut self, key: &StorageKey, value: &StorageValue, expiration: Option<u64>
    ) -> Result<InterpreterResponse, ServerError> {
        let element = StorageElement{key, value, expiration};
        let result = self.storage.set_if_not_exists(key, element)?;
        Ok(InterpreterResponse::Bool(result))
    }

    fn update(
        &mut self, key: &StorageKey, value: &StorageValue, expiration: Option<u64>
    ) -> Result<InterpreterResponse, ServerError> {
        let element = StorageElement{key, value, expiration};
        let result = self.storage.update(key, element)?;
        Ok(InterpreterResponse::Message("Ok".to_string()))
    }

    fn update_expiration(
        &mut self, key: &StorageKey, expiration: Option<u64>
    ) -> Result<InterpreterResponse, ServerError> {
        let result = self.storage.update_expiration(key, expiration)?;
        Ok(InterpreterResponse::Message("Ok".to_string()))
    }

    fn get_vector_element(&mut self, key: &StorageKey) -> Result<StorageVector, ServerError> {
        let element = self.storage.get(key)?;
        if let StorageValue::Vector(vector) = element.value {
            Ok(vector)
        } else {
            Err(ServerError::TypeError(format!("Element with key '{}' not a vector.", key)))
        }
    }

    fn get_map_element(&mut self, key: &StorageKey) -> Result<StorageMap, ServerError> {
        let element = self.storage.get(key)?;
        if let StorageValue::Map(map) = element.value {
            Ok(map)
        } else {
            Err(ServerError::TypeError(format!("Element with key '{}' not a map.", key)))
        }
    } 

    fn get_vector_element_mut(&mut self, key: &StorageKey) -> Result<&mut StorageVector, ServerError> {
        let element = self.storage.get_mut(key)?;
        if let StorageValue::Vector(&mut vector) = element.value {
            Ok(vector)
        } else {
            Err(ServerError::TypeError(format!("Element with key '{}' not a vector.", key)))
        }
    }

    fn get_map_element_mut(&mut self, key: &StorageKey) -> Result<&mut StorageMap, ServerError> {
        let element = self.storage.get_mut(key)?;
        if let StorageValue::Map(&mut map) = element.value {
            Ok(map)
        } else {
            Err(ServerError::TypeError(format!("Element with key '{}' not a map.", key)))
        }
    } 

    fn vector_get(
        &mut self, key: &StorageKey, index: usize
    ) -> Result<InterpreterResponse, ServerError> {
        let vector = self.get_vector_element(key)?;
        let value = vector.get(index)?;
        Ok(InterpreterResponse::Value(value.clone()))
    }
    
    fn vector_length(
        &mut self, key: &StorageKey
    ) -> Result<InterpreterResponse, ServerError> {
        let vector = self.get_vector_element(key)?;
        Ok(InterpreterResponse::Size(vector.len()))
    }

    fn vector_append(
        &mut self, key: &StorageKey, value: StorageValue
    ) -> Result<InterpreterResponse, ServerError> {
        let vector = self.get_vector_element_mut(key)?;
        vector.push(value)?;
        Ok(InterpreterResponse::Message("Ok".to_string()))
    }

    fn vector_pop(
        &mut self, key: &StorageKey
    ) -> Result<InterpreterResponse, ServerError> {
        let vector = self.get_vector_element_mut(key)?;
        let value = vector.pop();
        let value_response = match value {
            None => StorageValue::Null,
            Some(value) => value,
        };
        Ok(InterpreterResponse::Value(value))
    }

    fn vector_set(
        &mut self, key: &StorageKey, index: usize, value: StorageValue
    ) -> Result<InterpreterResponse, ServerError> {
        let vector = self.get_vector_element_mut(key)?;
        vector.set(index, value)?;
        Ok(InterpreterResponse::Message("Ok".to_string()))
    }

    fn map_get(
        &mut self, key: &StorageKey, map_key: &StorageValue
    ) -> Result<InterpreterResponse, ServerError> {
        let map = self.get_map_element(key)?;
        let value = map.get(map_key)?;
        Ok(InterpreterResponse::Value(value.clone()))
    }

    fn map_length(
        &mut self, key: &StorageKey
    ) -> Result<InterpreterResponse, ServerError> {
        let map = self.get_map_element(key)?;
        Ok(InterpreterResponse::Size(map.len()))
    }

    fn map_exists(
        &mut self, key: &StorageKey, map_key: &StorageValue
    ) -> Result<InterpreterResponse, ServerError> {
        let map = self.get_map_element(key)?;
        let result = map.contains_key(map_key)?;
        Ok(InterpreterResponse::Bool(result))
    }

    
    fn map_set(
        &mut self, key: &StorageKey, map_key: StorageValue, value: StorageValue
    ) -> Result<InterpreterResponse, ServerError> {
        let map = self.get_map_element_mut(key)?;
        let result = map.set(map_key, value)?;
        Ok(InterpreterResponse::Message("Ok".to_string()))
    }

    
    fn map_delete(
        &mut self, key: &StorageKey, map_key: &StorageValue
    ) -> Result<InterpreterResponse, ServerError> {
        let map = self.get_map_element_mut(key)?;
        let result = map.delete(map_key)?;
        Ok(InterpreterResponse::Bool(result))
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
