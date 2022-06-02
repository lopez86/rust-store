use std::time::SystemTime;
use client::data_types::{StorageKey, StorageValue};
use client::error::ServerError;


/// The types supported by the database
#[derive(Clone, Debug)]
pub enum StorageValue {
    /// Represents a key that exists but has no information
    NullValue,
    /// A boolean value
    BoolValue(bool),
    /// A single string
    StringValue(String),
    /// An integer
    IntValue(Int),
    /// A float
    FloatValue(Float),
    /// A list of strings
    StringVector(Vec<String>),
    /// A list of ints
    IntVector(Vec<Int>),
    /// A list of floats
    FloatVector(Vec<Float>),
    /// A map
    Map()
}


/// A storage element includes the key, the value, and an optional expiration time
#[derive(Clone)]
pub struct StorageElement {
    /// The key used to retrieve the value
    pub key: StorageKey,
    /// The expiration time, if any, for this entry
    pub expiration: Option<SystemTime>,
    /// The value pointed to by the key
    pub value: StorageValue,
}


impl StorageElement {
    /// Check if an element has expired already
    pub fn is_expired(&self) -> bool {
         match self.expiration {
            None => false,
            Some(expiration) => SystemTime::now() > expiration,
        }
    }
}


/// Create an error if a key was not found when it was expected to.
pub fn make_key_error(key: &str) -> ServerError {
    ServerError::KeyError(format!("No entry with key '{}' exists", key))
}


/// Create an error if a key was found when it wasn't expected to.
pub fn make_key_exists_error(key: &str) -> ServerError {
    ServerError::KeyError(format!("Entry with key '{}' already exists", key))
}


/// Functions needed to define a storage container.
/// 
/// This primarily uses basic get, set, delete, along 
///
/// There is also a get_random_key() function. This is there so that a thread can
/// be run to check and process expiration times. Random checking is going to be more predictable
/// in terms of how long it will take and how much memory it will use, since looping through the entire 
/// database could be quite expensive in some cases.
pub trait Storage {
    /// Gets the value for a key.
    fn get(&self, key: &str) -> Result<StorageElement, ServerError>;
    /// Sets the value for a key.
    fn set(&mut self, key: &str, value: StorageElement) -> Result<(), ServerError>;
    /// Gets a random key.
    fn get_random_key(&self) -> StorageKey;
    /// Check if a key exists in the database.
    fn exists(&self, key: &str) -> Result<bool, ServerError>;
    /// Get a value if it exists or else return None
    fn get_if_exists(&self, key: &str) -> Result<Option<StorageElement>, ServerError>;
    /// Set a value only if the key does not exist, else throw an error
    fn set_if_not_exists(&mut self, key: &str, value: StorageElement) -> Result<(), ServerError>;
    /// Update the value for an existing key.
    fn update(&mut self, key: &str, value: StorageElement) -> Result<(), ServerError>;
    /// Delete the contents of an existing key.
    fn delete(&mut self, key: &str) -> Result<(), ServerError>;
    /// Update the expiration time of an existing key.
    fn update_expiration(
        &mut self, key: &str, expiration: Option<SystemTime>
    ) -> Result<(), ServerError>;
}
