use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

use client::data_types::{Int, Float, StorageKey};
use client::error::ServerError;


/// Types of keys that can be used in a map
#[derive(Clone, Copy, Debug)]
pub enum KeyType{
    /// A string
    StringKey,
    /// An integer
    IntKey,
}

/// Types of values that can be saved in collections (Maps and Vectors)
#[derive(Clone, Copy, Debug)]
pub enum CollectionType {
    /// A collection of booleans
    BoolCollection,
    /// A collection of strings
    StringCollection,
    /// A collection of integers
    IntCollection,
    /// A collection of floats
    FloatCollection,
}


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
    /// A vector
    VectorValue(StorageVector),
    /// A map
    MapValue(StorageMap),
}

impl Hash for StorageValue {
    /// Hash function for StorageValue instances
    /// 
    /// This is only defined for StringValue and IntValue, otherwise will panic.
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher
    {
        match self {
            StorageValue::StringValue(value) => value.hash(state),
            StorageValue::IntValue(value) => value.hash(state),
            _ => unimplemented!("Hash only implemented for StorageValues IntValue and FloatValue."),
        };
    }
}

impl PartialEq for StorageValue {
    /// Equality for StorageValue is only defined for BoolValue, StringValue, and IntValue, all else
    /// will return false.
    fn eq(&self, other: &Self) -> bool {
        match self {
            StorageValue::BoolValue(value) => {
                if let StorageValue::BoolValue(other_value) = other {
                    value == other_value
                } else {
                    false
                }
            },
            StorageValue::StringValue(value) => {
                if let StorageValue::StringValue(other_value) = other {
                    value == other_value
                } else {
                    false
                }
            },
            StorageValue::FloatValue(value) => {
                if let StorageValue::FloatValue(other_value) = other {
                    value == other_value
                } else {
                    false
                }
            },
            _ => false
        }
    }
}


impl Eq for StorageValue {}


/// Check that a storage value matches the expected type
fn validate_value(
    value: &StorageValue, value_type: CollectionType
) -> Result<(), ServerError> {
    match value_type {
        CollectionType::BoolCollection => {
            match value {
                StorageValue::IntValue(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
        CollectionType::IntCollection => {
            match value {
                StorageValue::IntValue(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
        CollectionType::StringCollection => {
            match value {
                StorageValue::StringValue(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
        CollectionType::FloatCollection => {
            match value {
                StorageValue::FloatValue(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
    }
}


/// Check that a storage value matches the expected key type
fn validate_key(key: &StorageValue, key_type: KeyType) -> Result<(), ServerError> {
    match key_type {
        KeyType::IntKey => {
            match key {
                StorageValue::IntValue(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
        KeyType::StringKey => {
            match key {
                StorageValue::StringValue(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected a string key.".to_string()))
            }
        }
    }
}

/// A vector object that can be saved in the key value store
#[derive(Clone, Debug)]
pub struct StorageVector {
    /// The raw vector to be accessed
    vector: Vec<StorageValue>,
    /// The type of data held in the vector
    value_type: CollectionType
}



impl StorageVector {
    /// Create a new vector holding some data type
    pub fn new(value_type: CollectionType) -> StorageVector {
        StorageVector{vector: vec![], value_type}
    }

    /// Pop the last value off the vector and return it
    pub fn pop(&mut self) -> Option<StorageValue> {
        self.vector.pop()
    }

    /// Get the length of the vector
    pub fn len(&self) -> usize {
        self.vector.len()
    }

    /// Get the value at the given location
    pub fn get(&self, index: usize) -> Result<&StorageValue, ServerError> {
        match self.vector.get(index) {
            Some(value) => Ok(value),
            None => Err(
                ServerError::IndexError(
                    format!(
                        "Cannot get entry {}, vector has only {} elements.",
                        index,
                        self.vector.len(),
                    )
                )
            ),
        }
    }

    /// Push a new value to the end of the vector
    pub fn push(&mut self, value: StorageValue) -> Result<(), ServerError> {
        match validate_value(&value, self.value_type) {
            Ok(_) => (),
            Err(err) => return Err(err)
        };
        self.vector.push(value);
        Ok(())
    }

    /// Set the value at a given index
    pub fn set(&mut self, index: usize, value: StorageValue) -> Result<(), ServerError> {
        match validate_value(&value, self.value_type) {
            Ok(_) => (),
            Err(err) => return Err(err)
        };
        if index >= self.vector.len() {
            return Err(
                ServerError::IndexError(
                    format!(
                        "Cannot set value at index {}. Vector has {} elements.",
                        index,
                        self.vector.len(),
                    )
                )
            )
        }
        self.vector[index] = value;
        Ok(())
    }
}



/// A map object that can be saved in the key value store
#[derive(Clone, Debug)]
pub struct StorageMap {
    /// The raw map to be accessed
    map: HashMap<StorageValue, StorageValue>,
    /// The type of key to be used
    key_type: KeyType,
    /// The type of data held in the map
    value_type: CollectionType,    
}


impl StorageMap {
    /// Create a new map with keys and data of the given types.
    pub fn new(key_type: KeyType, value_type: CollectionType) -> StorageMap {
        StorageMap{map: HashMap::new(), key_type, value_type}
    }

    /// Get the value with the given key. 
    /// 
    /// Returns an error if:
    ///   1) An incorrect key type is found (TypeError)
    ///   2) The key is not present (KeyError)
    pub fn get(&self, key: &StorageValue) -> Result<&StorageValue, ServerError> {
        let value = match validate_key(key, self.key_type) {
            Ok(_) => self.map.get(key),
            Err(err) => return Err(err),
        };
        match value {
            Some(value) => Ok(value),
            None => Err(ServerError::IndexError("No entry found for the given key.".to_string()))
        }
    }

    
    /// See if the given key exists in the map
    /// 
    /// Returns an error if an incorrect key type is found (TypeError)
    pub fn contains_key(&self, key: &StorageValue) -> Result<bool, ServerError> {
        let value = match validate_key(key, self.key_type) {
            Ok(_) => self.map.contains_key(key),
            Err(err) => return Err(err),
        };
        Ok(value)
    }


    /// See if the given key exists in the map
    /// 
    /// Returns an error if:
    ///   1) An incorrect key type is found (TypeError)
    ///   2) An incorrect value type is found (TypeError)
    pub fn set(
        &mut self, key: StorageValue, value: StorageValue
    ) -> Result<(), ServerError> {
        match validate_key(&key, self.key_type) {
            Ok(_) => (),
            Err(err) => return Err(err),
        };
        match validate_value(&value, self.value_type) {
            Ok(_) => (),
            Err(err) => return Err(err)
        };

        self.map.insert(key, value);
        Ok(())
    }

    /// Remove an entry from the map
    pub fn delete(&mut self, key: &StorageValue) -> Result<bool, ServerError>{
        match validate_key(&key, self.key_type) {
            Ok(_) => (),
            Err(err) => return Err(err),
        };
        match self.map.remove_entry(key) {
            Some(_) => Ok(true),
            None => Ok(false)
        }
    }

    /// Get the number of entries in the map
    pub fn len(&self) -> usize {
        self.map.len()
    }

    
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


#[cfg(test)]
mod test {
    use super::{*};

    #[test]
    fn test_validate_key() {
        assert!(
            matches!(
                validate_key(&StorageValue::IntValue(0), KeyType::IntKey),
                Ok(()),
            )
        );
        assert!(
            matches!(
                validate_key(&StorageValue::StringValue(String::from("str")), KeyType::StringKey),
                Ok(()),
            )
        );
        assert!(
            matches!(
                validate_key(&StorageValue::IntValue(0), KeyType::StringKey),
                Err(_),
            )
        );
        assert!(
            matches!(
                validate_key(&StorageValue::BoolValue(false), KeyType::StringKey),
                Err(_),
            )
        );
        assert!(
            matches!(
                validate_key(&StorageValue::StringValue(String::from("str")), KeyType::IntKey),
                Err(_),
            )
        );
    }

    #[test]
    fn test_validate_value() {
        
    }
}
