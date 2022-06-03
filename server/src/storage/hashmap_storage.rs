use std::collections::HashMap;
use std::vec::Vec;
use std::time::SystemTime;

use rand;
use rand::RngCore;

use crate::storage::types::{
    Storage,
    StorageElement,
    make_key_error,
    make_key_exists_error,
};
use client::data_types::StorageKey;
use client::error::ServerError;


/// Container for an entry in the hash map.
struct HashMapContainer {
    /// The data for an entry in the database
    element: StorageElement,
    /// The location in the key vector for O(1) time deletion
    key_index: usize,
}


/// Top level storage container backed by a HashMap
/// A vector of keys is provided to allow for O(1) time 
/// random access.
pub struct HashMapStorage {
    storage: HashMap<StorageKey, HashMapContainer>,
    keys: Vec<StorageKey>,
}


/// Implement the Storage trait for the HashMapStorage
impl Storage for HashMapStorage {

    /// Get a value if it exists.
    fn get_if_exists(&self, key: &str) -> Result<Option<StorageElement>, ServerError> {
        match self.storage.get(key) {
            Some(value) if value.element.is_expired() => Err(make_key_error(key)),
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

    /// Update the expiration time of an entry.
    fn update_expiration(
        &mut self, key: &str, expiration: Option<SystemTime>
    ) -> Result<(), ServerError> {
        let old_container = self.storage.get_mut(key);
        
        let result = match old_container {
            Some(container) if container.element.is_expired() => {
                Err(make_key_error(key))
            },
            Some(container) => {
                container.element.expiration = expiration;
                Ok(())
            },
            None => Err(make_key_error(key))
        };
        result
    }

    /// Get a random key from the database.
    fn get_random_key(&self) -> StorageKey {
        let mut rng = rand::thread_rng();
        let index = (rng.next_u32() as usize) % self.keys.len();
        self.keys[index].clone()
    }

    /// Delete an entry from the database.
    fn delete(
        &mut self, key: &str
    ) -> Result<(), ServerError> {
        let value = self.storage.remove(key);
        let result = if let Some(container) = value {
            let index = container.key_index;
            let last_key = self.keys[self.keys.len() - 1].clone();
            self.keys.swap_remove(index);
            let moved_container = self.storage.get_mut(&last_key).unwrap();
            moved_container.key_index = index;
            if container.element.is_expired() {
                Err(make_key_error(key))
            } else {
                Ok(())
            }
        } else {
            Err(make_key_error(key))
        };
        result
    }

    /// Update an existing entry.
    fn update(
        &mut self, key: &str, value: StorageElement
    ) -> Result<(), ServerError> {
        if !self.storage.contains_key(key) {
            return Err(make_key_error(key))
        }
        self.set(key, value)
    }

    /// Set a value if it doesn't already exist.
    fn set_if_not_exists(
        &mut self, key: &str, value: StorageElement
    ) -> Result<(), ServerError> {
        if self.storage.contains_key(key) {
            Err(make_key_exists_error(key))
        } else {
            self.set(key, value)
        }
    }


    /// Set a value.
    fn set(
        &mut self, key: &str, value: StorageElement
    ) -> Result<(), ServerError> {
        self.keys.push(String::from(key));
        let index = self.keys.len();
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
    fn exists(&self, key: &str) -> Result<bool, ServerError> {
        Ok(self.storage.contains_key(key))
    }
}


#[cfg(test)]
mod tests {

}
