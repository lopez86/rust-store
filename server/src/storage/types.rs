use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

use crate::error::ServerError;


/// Type alias for the key type
pub type StorageKey = String;
/// Type alias for internal floating point types
pub type Float = f32;
/// Type alias for internal integer types
pub type Int = i64;



/// Types of keys that can be used in a map
#[derive(Clone, Copy, Debug)]
pub enum KeyType{
    /// A string
    String,
    /// An integer
    Int,
}

/// Types of values that can be saved in collections (Maps and Vectors)
#[derive(Clone, Copy, Debug)]
pub enum CollectionType {
    /// A collection of booleans
    Bool,
    /// A collection of strings
    String,
    /// A collection of integers
    Int,
    /// A collection of floats
    Float,
}


/// The types supported by the database
#[derive(Clone, Debug)]
pub enum StorageValue {
    /// Represents a key that exists but has no information
    Null,
    /// A boolean value
    Bool(bool),
    /// A single string
    String(String),
    /// An integer
    Int(Int),
    /// A float
    Float(Float),
    /// A vector
    Vector(StorageVector),
    /// A map
    Map(StorageMap),
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
            StorageValue::String(value) => (*value).hash(state),
            StorageValue::Int(value) => (*value).hash(state),
            _ => unimplemented!("Hash only implemented for StorageValues IntValue and FloatValue."),
        };
    }
}

impl PartialEq for StorageValue {
    /// Equality for StorageValue is only defined for BoolValue, StringValue, and IntValue, all else
    /// will return false.
    fn eq(&self, other: &Self) -> bool {
        match self {
            StorageValue::Bool(value) => {
                if let StorageValue::Bool(other_value) = other {
                    value == other_value
                } else {
                    false
                }
            },
            StorageValue::Int(value) => {
                if let StorageValue::Int(other_value) = other {
                    value == other_value
                } else {
                    false
                }
            },
            StorageValue::String(value) => {
                if let StorageValue::String(other_value) = other {
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
    value: &StorageValue, collection_type: CollectionType
) -> Result<(), ServerError> {
    match collection_type {
        CollectionType::Bool => {
            match value {
                StorageValue::Bool(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
        CollectionType::Int => {
            match value {
                StorageValue::Int(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
        CollectionType::String => {
            match value {
                StorageValue::String(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
        CollectionType::Float => {
            match value {
                StorageValue::Float(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
    }
}


/// Check that a storage value matches the expected key type
fn validate_key(key: &StorageValue, key_type: KeyType) -> Result<(), ServerError> {
    match key_type {
        KeyType::Int => {
            match key {
                StorageValue::Int(_) => Ok(()),
                _ => Err(ServerError::TypeError("Expected an integer key".to_string())),
            }
        },
        KeyType::String => {
            match key {
                StorageValue::String(_) => Ok(()),
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
    collection_type: CollectionType
}



impl StorageVector {
    /// Create a new vector holding some data type
    pub fn new(collection_type: CollectionType) -> StorageVector {
        StorageVector{vector: vec![], collection_type}
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
        match validate_value(&value, self.collection_type) {
            Ok(_) => (),
            Err(err) => return Err(err)
        };
        self.vector.push(value);
        Ok(())
    }

    /// Set the value at a given index
    pub fn set(&mut self, index: usize, value: StorageValue) -> Result<(), ServerError> {
        match validate_value(&value, self.collection_type) {
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
    collection_type: CollectionType,    
}


impl StorageMap {
    /// Create a new map with keys and data of the given types.
    pub fn new(key_type: KeyType, collection_type: CollectionType) -> StorageMap {
        StorageMap{map: HashMap::new(), key_type, collection_type}
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
        match validate_value(&value, self.collection_type) {
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
#[derive(Clone, Debug)]
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
    /// Runs the policy on invalidating expired keys
    fn invalidate_expired_keys(&mut self) -> Result<usize, ServerError>;
    /// Check if a key exists in the database.
    fn contains_key(&self, key: &str) -> Result<bool, ServerError>;
    /// Get a value if it exists or else return None
    fn get_if_exists(&self, key: &str) -> Result<Option<StorageElement>, ServerError>;
    /// Set a value only if the key does not exist
    fn set_if_not_exists(&mut self, key: &str, value: StorageElement) -> Result<bool, ServerError>;
    /// Update the value for an existing key.
    fn update(&mut self, key: &str, value: StorageElement) -> Result<(), ServerError>;
    /// Delete the contents of an existing key.
    fn delete(&mut self, key: &str) -> Result<bool, ServerError>;
    /// Update the expiration time of an existing key.
    fn update_expiration(
        &mut self, key: &str, expiration: Option<SystemTime>
    ) -> Result<(), ServerError>;
    /// Get the number of keys
    fn len(&self) -> Result<usize, ServerError>;
    /// Remove a key if it's expired
    fn check_and_expire(&mut self, key: &str) -> Result<bool, ServerError>;
    /// Get the number of expiring keys
    fn expiring_keys_count(&self) -> Result<usize, ServerError>;
}


#[cfg(test)]
mod test {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    fn calculate_hash<T: Hash,>(t: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_hash_storage_value_int() {      
        let _five_hash = calculate_hash(&StorageValue::Int(5));
        let _five_hash_2 = calculate_hash(&StorageValue::Int(5));
        let _three_hash = calculate_hash(&StorageValue::Int(3));
        assert_eq!(_five_hash, _five_hash_2);
        assert_ne!(_five_hash, _three_hash);
    }

    #[test]
    fn test_hash_storage_value_string() {
        let _str_hash = calculate_hash(&StorageValue::String("str".to_string()));
        let _str_hash_2 = calculate_hash(&StorageValue::String("str".to_string()));
        let _str2_hash = calculate_hash(&StorageValue::String("str2".to_string()));
        assert_eq!(_str_hash, _str_hash_2);
        assert_ne!(_str_hash, _str2_hash);
    }

    #[test]
    #[should_panic]
    fn test_hash_storage_value_other() {
        calculate_hash(
            &StorageValue::Vector(StorageVector::new(CollectionType::Bool))
        );
    }

    #[test]
    fn test_storage_value_equality() {
        let x = StorageValue::Int(5);
        let y = StorageValue::Int(6);
        let z = StorageValue::Int(5);

        assert_eq!(x == y, false);
        assert_eq!(x == z, true);
    }

    #[test]
    fn test_validate_key_with_good_inputs() {
        assert!(matches!(validate_key(&StorageValue::Int(0), KeyType::Int), Ok(())));
        assert!(
            matches!(
                validate_key(&StorageValue::String(String::from("str")), KeyType::String),
                Ok(()),
            )
        );
    }



    #[test]
    fn test_validate_key_with_mismatched_inputs() {
        assert!(matches!(validate_key(&StorageValue::Int(0), KeyType::String), Err(_)));
        assert!(
            matches!(validate_key(&StorageValue::Bool(false), KeyType::String), Err(_))
        );
        assert!(
            matches!(
                validate_key(&StorageValue::String(String::from("str")), KeyType::Int),
                Err(_),
            )
        );
    }

    #[test]
    fn test_validate_value_with_good_inputs() {
        assert!(
            matches!(validate_value(&StorageValue::Int(0), CollectionType::Int), Ok(()))
        );
        assert!(
            matches!(
                validate_value(&StorageValue::Float(0.), CollectionType::Float),
                Ok(()),
            )
        );
        assert!(
            matches!(
                validate_value(
                    &StorageValue::String("str".to_string()),
                    CollectionType::String,
                ),
                Ok(())
            )
        );
        assert!(
            matches!(
                validate_value(&StorageValue::Bool(true), CollectionType::Bool),
                Ok(()),
            )
        );

    }

    #[test]
    fn test_validate_value_with_bad_inputs() {
        assert!(
            matches!(validate_value(&StorageValue::Int(0), CollectionType::Bool), Err(_))
        );
        assert!(
            matches!(validate_value(&StorageValue::Float(0.), CollectionType::Int), Err(_))
        );
        assert!(
            matches!(
                validate_value(
                    &StorageValue::String("str".to_string()),
                    CollectionType::Float,
                ),
                Err(_)
            )
        );
        assert!(
            matches!(validate_value(&StorageValue::Bool(true), CollectionType::String), Err(_))
        );
    }

    #[test]
    fn test_vector_new() {
        let vector = StorageVector::new(CollectionType::Int);
        assert!(matches!(vector.collection_type, CollectionType::Int));
        assert_eq!(vector.vector.len(), 0);
    }

    #[test]
    fn test_vector_push() {
        let mut vector = StorageVector::new(CollectionType::Bool);
        vector.push(StorageValue::Bool(true)).unwrap();
        vector.push(StorageValue::Bool(false)).unwrap();
        assert_eq!(vector.vector.len(), 2);
    }

    #[test]
    fn test_vector_push_with_bad_value() {
        let mut vector = StorageVector::new(CollectionType::Bool);
        let result = vector.push(StorageValue::Int(0));
        assert!(matches!(result, Err(_)));
    }

    #[test]
    fn test_vector_len() {
        let mut vector = StorageVector::new(CollectionType::Int);
        assert_eq!(vector.len(), 0);
        vector.push(StorageValue::Int(0)).unwrap();
        assert_eq!(vector.len(), 1);
    }

    #[test]
    fn test_vector_pop() {
        let mut vector = StorageVector::new(CollectionType::Int);
        vector.push(StorageValue::Int(5)).unwrap();
        vector.push(StorageValue::Int(8)).unwrap();
        assert_eq!(vector.len(), 2);
        assert!(matches!(vector.pop(), Some(StorageValue::Int(8))));
        assert!(matches!(vector.pop(), Some(StorageValue::Int(5))));
        assert!(matches!(vector.pop(), None));
    }

    #[test]
    fn test_vector_get() {
        let mut vector = StorageVector::new(CollectionType::String);
        vector.push(StorageValue::String("hello".to_string())).unwrap();
        vector.push(StorageValue::String("hi".to_string())).unwrap();
        let _element_0 = StorageValue::String("hello".to_string());
        let _element_1 = StorageValue::String("hi".to_string());
        assert!(matches!(vector.get(0).unwrap(), _element_0));
        assert!(matches!(vector.get(1).unwrap(), _element_1));
        assert!(matches!(vector.get(2), Err(ServerError::IndexError(_))));
    }

    #[test]
    fn test_vector_set() {
        let mut vector = StorageVector::new(CollectionType::Int);
        vector.push(StorageValue::Int(1)).unwrap();
        vector.push(StorageValue::Int(4)).unwrap();
        vector.push(StorageValue::Int(3)).unwrap();

        vector.set(1, StorageValue::Int(8)).unwrap();
        assert!(matches!(vector.get(0).unwrap(), StorageValue::Int(1)));
        assert!(matches!(vector.get(1).unwrap(), StorageValue::Int(8)));
        assert!(matches!(vector.get(2).unwrap(), StorageValue::Int(3)));
        assert!(matches!(vector.get(3), Err(ServerError::IndexError(_))));
    }

    #[test]
    fn test_map_new() {
        let map = StorageMap::new(KeyType::Int, CollectionType::Float);
        assert_eq!(map.map.len(), 0);
        assert!(matches!(map.key_type, KeyType::Int));
        assert!(matches!(map.collection_type, CollectionType::Float));
    }

    #[test]
    fn test_map_set() {
        let mut map = StorageMap::new(KeyType::Int, CollectionType::Bool);
        map.set(StorageValue::Int(5), StorageValue::Bool(true)).unwrap();
        assert_eq!(map.map.len(), 1);
        assert!(matches!(map.map.get(&StorageValue::Int(5)).unwrap(), StorageValue::Bool(true)));
    }

    #[test]
    fn test_map_get() {
        let mut map = StorageMap::new(KeyType::String, CollectionType::String);
        map.set(
            StorageValue::String("key".to_string()),
            StorageValue::String("value".to_string()),
        ).unwrap();
        let _expected_string = StorageValue::String("value".to_string());
        assert!(
            matches!(
                map.get(&StorageValue::String("key".to_string())).unwrap(),
                _expected_string,
            )
        );
        assert!(
            matches!(
                map.get(&StorageValue::String("key2".to_string())),
                Err(ServerError::IndexError(_)),
            )
        );
    }

    #[test]
    fn test_map_len() {
        let mut map = StorageMap::new(KeyType::Int, CollectionType::Bool);
        map.set(StorageValue::Int(5), StorageValue::Bool(true)).unwrap();
        assert_eq!(map.len(), 1);
        map.set(StorageValue::Int(55), StorageValue::Bool(true)).unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_map_contains_key() {
        let mut map = StorageMap::new(KeyType::String, CollectionType::Float);
        map.set(StorageValue::String("key".to_string()), StorageValue::Float(1.5)).unwrap();
        assert_eq!(map.contains_key(&StorageValue::String("key".to_string())).unwrap(), true);
        assert_eq!(map.contains_key(&StorageValue::String("key2".to_string())).unwrap(), false);
        assert!(matches!(map.contains_key(&StorageValue::Int(0)), Err(ServerError::TypeError(_))));
    }

    #[test]
    fn test_map_delete() {
        let mut map = StorageMap::new(KeyType::String, CollectionType::Float);
        map.set(StorageValue::String("key".to_string()), StorageValue::Float(1.5)).unwrap();
        assert_eq!(map.delete(&StorageValue::String("key".to_string())).unwrap(), true);
        assert_eq!(map.delete(&StorageValue::String("key".to_string())).unwrap(), false);
        assert!(matches!(map.delete(&StorageValue::Int(0)), Err(ServerError::TypeError(_))));

    }


}
