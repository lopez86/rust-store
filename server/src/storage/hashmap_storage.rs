use std::collections::HashMap;
use std::vec::Vec;
use std::time::SystemTime;

use rand;
use rand::RngCore;

use crate::error::ServerError;
use crate::storage::{
    Storage,
    StorageElement,
    StorageKey,
    make_key_error,
};


/// Container for an entry in the hash map.
#[derive(Debug)]
struct HashMapContainer {
    /// The data for an entry in the database
    element: StorageElement,
    /// The location in the key vector for O(1) time deletion
    key_index: Option<usize>,
}


/// Top level storage container backed by a HashMap
/// A vector of keys is provided to allow for O(1) time 
/// random access.
pub struct HashMapStorage {
    storage: HashMap<StorageKey, HashMapContainer>,
    expiring_keys: Vec<StorageKey>,
}

impl HashMapStorage {
    /// Create a new storage container
    pub fn new() -> HashMapStorage {
        HashMapStorage { storage: HashMap::new(), expiring_keys: vec![] }
    }

    fn invalidate_key_index(&mut self, index: usize) {
        let removing_last = index == self.expiring_keys.len() - 1;
        if !removing_last {
            let moved_container = self.storage.get_mut(
                &self.expiring_keys[self.expiring_keys.len() - 1]
            ).unwrap();
            moved_container.key_index = Some(index);
        }
        // O(1) removal but does not preserve order - swap + pop from end
        self.expiring_keys.swap_remove(index);

    }

    /// Get a random key from the database.
    fn get_random_key(&self) -> Option<&StorageKey> {
        if self.expiring_keys.len() == 0 {
            return None
        }
        let mut rng = rand::thread_rng();
        let index = (rng.next_u64() as usize) % self.expiring_keys.len();
        Some(&self.expiring_keys[index])
    }
}


/// Implement the Storage trait for the HashMapStorage
impl Storage for HashMapStorage {

    /// Get a value if it exists.
    fn get_if_exists(&self, key: &str) -> Result<Option<StorageElement>, ServerError> {
        match self.storage.get(key) {
            Some(value) if value.element.is_expired() => Ok(None),
            Some(value) => Ok(Some(value.element.clone())),
            None => Ok(None),
        }
    }

    /// Get a value or else throw an error.
    fn get(&self, key: &str) -> Result<StorageElement, ServerError> {
        match self.get_if_exists(key) {
            Ok(Some(element)) => Ok(element.clone()),
            Ok(None) => Err(make_key_error(key)),
            Err(error) => Err(error),
        }
    }

    /// Get a value or else throw an error.
    fn get_mut(&mut self, key: &str) -> Result<&mut StorageElement, ServerError> {
        match self.storage.get_mut(key) {
            Some(value) if value.element.is_expired() => Err(make_key_error(key)),
            Some(value) =>Ok(&mut value.element),
            None => Err(ServerError::KeyError(format!("No item with index {} found.", key))),
        }
    }

    /// Update the expiration time of an entry.
    fn update_expiration(
        &mut self, key: &str, expiration: Option<SystemTime>
    ) -> Result<(), ServerError> {        
        let new_key_index = match self.storage.get(key) {
            Some(container) if container.element.is_expired() => {
                return Err(make_key_error(key))
            },
            Some(container) => {
                if let Some(_) = container.element.expiration {
                    // Need to remove from expiring keys
                    if let None = expiration {
                        let index = container.key_index.unwrap();
                        self.invalidate_key_index(index);
                        None
                    } else {
                        container.key_index
                    }
                } else {
                    // Need to add to expiring keys
                    if let Some(_) = expiration {
                        self.expiring_keys.push(key.to_string());
                        Some(self.expiring_keys.len() - 1)
                    } else {
                        None
                    }
                }
            },
            None => return Err(make_key_error(key))
        };
        let container = self.storage.get_mut(key).unwrap();
        container.element.expiration = expiration;
        container.key_index = new_key_index;

        Ok(())
    }

    /// Get a random key from the database.
    fn invalidate_expired_keys(&mut self) -> Result<usize, ServerError> {
        let key = match self.get_random_key() {
            Some(key) => key.clone(),
            None => return Ok(0),
        };
        match self.check_and_expire(&key) {
            Ok(true) => Ok(1),
            Ok(false) => Ok(0),
            Err(err) => Err(err),
        }
    }

    /// Delete an entry from the database.
    fn delete(
        &mut self, key: &str
    ) -> Result<bool, ServerError> {
        let value = self.storage.remove(key);
        println!("{:?}", value);
        let result = if let Some(container) = value {
            if let Some(_) = container.element.expiration {
                let index = container.key_index.unwrap();
                self.invalidate_key_index(index);
            }
            if container.element.is_expired() {
                Ok(false)
            } else {
                Ok(true)
            }
        } else {
            Ok(false)
        };
        result
    }

    /// Update an existing entry.
    fn update(
        &mut self, key: &str, value: StorageElement
    ) -> Result<(), ServerError> {
        match self.contains_key(key) {
            Ok(false) => Err(make_key_error(key)),
            Ok(true) => self.set(key, value),
            Err(err) => Err(err),
        }
    }

    /// Set a value if it doesn't already exist.
    fn set_if_not_exists(
        &mut self, key: &str, value: StorageElement
    ) -> Result<bool, ServerError> {
        if self.storage.contains_key(key) {
            Ok(false)
        } else {
            match self.set(key, value) {
                Ok(()) => Ok(true),
                Err(other) => Err(other), 
            }
        }
    }

    /// Set a value.
    fn set(
        &mut self, key: &str, value: StorageElement
    ) -> Result<(), ServerError> {
        let index = match self.storage.get(key) {
            None => {
                if let None = value.expiration {
                    None
                } else {
                    self.expiring_keys.push(String::from(key));
                    Some(self.expiring_keys.len() - 1)
                }
            } 
            Some(container) => {
                match container.key_index {
                    None => {
                        if let None = value.expiration {
                            None
                        } else {
                            self.expiring_keys.push(String::from(key));
                            Some(self.expiring_keys.len() - 1)
                        }
                    },
                    Some(key_index) => {
                        if let None = value.expiration {
                            self.invalidate_key_index(key_index);
                            None
                        } else {
                            Some(key_index)
                        }
                    }
                }
            }
        };
        self.storage.insert(
            StorageKey::from(key),
            HashMapContainer {
                element: value,
                key_index: index,
            }
        );
        Ok(())
    }

    /// Check if a key exists in the database.
    fn contains_key(&self, key: &str) -> Result<bool, ServerError> {
        match self.storage.get(key) {
            Some(container) => {
                Ok(!container.element.is_expired())
            },
            None => Ok(false)
        }
    }

    /// Get the number of keys
    fn len(&self) -> Result<usize, ServerError> {
        Ok(self.storage.len())
    }

    /// Check if a key is expired and remove if so
    fn check_and_expire(&mut self, key: &str) -> Result<bool, ServerError> {
        let item = match self.storage.get(key) {
            Some(container) => container,
            None => return Err(ServerError::KeyError(format!("Key {} not found.", key))),
        };
        if item.element.is_expired() {
            match self.storage.remove(key) {
                _ => Ok(true),
            }
        } else {
            Ok(false)
        }
    }

    /// Get the number of expiring keys
    fn expiring_keys_count(&self) -> Result<usize, ServerError> {
        Ok(self.expiring_keys.len())
    }
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::storage::types::{CollectionType, StorageValue, StorageVector};

    #[test]
    fn test_new() {
        let map = HashMapStorage::new();
        assert_eq!(map.storage.len(), 0);
        assert_eq!(map.expiring_keys.len(), 0);
    }

    #[test]
    fn test_set() {
        let mut storage = HashMapStorage::new();
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        let mut vector = StorageVector::new(CollectionType::Bool);
        vector.push(StorageValue::Bool(true)).unwrap();
        vector.push(StorageValue::Bool(false)).unwrap();
        let element2 = StorageElement {
            key: "key2".to_string(),
            value: StorageValue::Vector(vector),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        storage.set("key2", element2).unwrap();

        assert_eq!(storage.storage.len(), 2);
        assert_eq!(storage.storage.contains_key("key1"), true);
        assert_eq!(storage.storage.contains_key("key2"), true);
        storage.storage.get("key1").unwrap();
        storage.storage.get("key2").unwrap();
    }

    #[test]
    fn test_set_if_not_exists() {
        let mut storage = HashMapStorage::new();
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        let element2 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Bool(false),
            expiration: None,
        };
        assert_eq!(storage.set_if_not_exists("key1", element2).unwrap(), false);
        let element3 = StorageElement {
            key: "key2".to_string(),
            value: StorageValue::Int(11),
            expiration: None,
        };
        assert_eq!(storage.set_if_not_exists("key2", element3).unwrap(), true);
        assert_eq!(storage.storage.len(), 2);
        assert!(
            matches!(storage.storage.get("key1").unwrap().element.value, StorageValue::Int(13))
        );
        assert!(
            matches!(storage.storage.get("key2").unwrap().element.value, StorageValue::Int(11))
        );

    }

    #[test]
    fn test_get() {
        let mut storage = HashMapStorage::new();
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        assert!(matches!(storage.get("key1").unwrap().value, StorageValue::Int(13)));
        assert!(matches!(storage.get("unknown_key"), Err(ServerError::KeyError(_))));
    }

    #[test]
    fn test_get_if_exists() {
        let mut storage = HashMapStorage::new();
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        assert!(matches!(storage.get_if_exists("key1").unwrap().unwrap().value, StorageValue::Int(13)));
        assert!(matches!(storage.get_if_exists("unknown_key").unwrap(), None));
    }

    #[test]
    fn test_contains_key() {
        let mut storage = HashMapStorage::new();
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        assert_eq!(storage.contains_key("key1").unwrap(), true);
        assert_eq!(storage.contains_key("unknown_key").unwrap(), false);
    }

    #[test]
    fn test_update() {
        let mut storage = HashMapStorage::new();
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        let element2 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(15),
            expiration: None,
        };
        storage.update("key1", element2).unwrap();
        assert!(matches!(storage.get("key1").unwrap().value, StorageValue::Int(15)));
    }

    #[test]
    fn test_delete() {
        let mut storage = HashMapStorage::new();
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        let element2 = StorageElement {
            key: "key2".to_string(),
            value: StorageValue::Int(15),
            expiration: None,
        };
        storage.set("key2", element2).unwrap();
        assert_eq!(storage.delete("key2").unwrap(), true);
        assert_eq!(storage.delete("key2").unwrap(), false);
        assert_eq!(storage.contains_key("key1").unwrap(), true);
        assert_eq!(storage.contains_key("key2").unwrap(), false);
    }

    #[test]
    fn test_update_expiration() {
        let mut storage = HashMapStorage::new();
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        let new_expiration = SystemTime::now() + Duration::from_secs(5000);
        storage.update_expiration("key1", Some(new_expiration)).unwrap();
        assert!(matches!(storage.get("key1").unwrap().expiration, Some(_)));
        storage.update_expiration("key1", None).unwrap();
        assert!(matches!(storage.get("key1").unwrap().expiration, None));
        assert!(matches!(storage.update_expiration("bad_key", None), Err(ServerError::KeyError(_))));
    }

    #[test]
    fn test_get_random_key() {
        let mut storage = HashMapStorage::new();
        assert!(matches!(storage.get_random_key(), None));
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: None,
        };
        storage.set("key1", element1).unwrap();
        assert!(matches!(storage.get_random_key(), Some(_)))
    }

    #[test]
    fn test_expired_key() {
        let mut storage = HashMapStorage::new();
        assert!(matches!(storage.get_random_key(), None));
        // We'll allow setting an expired key
        // Will be disallowed by the user API
        let element1 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: Some(SystemTime::now() - Duration::from_secs(1)),
        };
        let element2 = StorageElement {
            key: "key1".to_string(),
            value: StorageValue::Int(13),
            expiration: Some(SystemTime::now() + Duration::from_secs(500)),
        };
        let new_expiration = Some(SystemTime::now() + Duration::from_secs(500));
        storage.set("key1", element1).unwrap();
        assert!(matches!(storage.get_if_exists("key1").unwrap(), None));
        assert!(matches!(storage.update("key1", element2), Err(ServerError::KeyError(_))));
        assert!(
            matches!(
                storage.update_expiration("key1", new_expiration),
                Err(ServerError::KeyError(_)),
            )
        );
        assert_eq!(storage.storage.len(), 1);
        assert_eq!(storage.delete("key1").unwrap(), false);
        assert_eq!(storage.storage.len(), 0);
    }
}
