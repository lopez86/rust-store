use std::fmt::Debug;

use client::data_types::StorageKey;

use crate::storage::StorageValue;

/// Lifetime in seconds of a 
type Lifetime = u64;


/// Statement
#[derive(Clone, Debug)]
pub enum Statement {
    /// Get a value
    Get(StorageKey),
    /// Set a value
    Set(StorageKey, StorageValue, Option<Lifetime>),
    /// Update an existing value
    Update(StorageKey, StorageValue, Option<Lifetime>),
    /// See if a key exists already
    Exists(StorageKey),
    /// Delete a value
    Delete(StorageKey),
    /// Get the lifetime of a value
    GetLifetime(StorageKey),
    /// Update the lifetime of a value
    UpdateLifetime(StorageKey, Option<Lifetime>),
    /// Get a value if it exists, else nothing
    GetIfExists(StorageKey),
    /// Set a value if it exists, else nothing
    SetIfNotExists(StorageKey, StorageValue, Option<Lifetime>),
    /// Get a value from a vector
    VectorGet(StorageKey, usize),
    /// Set a value in a vector
    VectorSet(StorageKey, usize, StorageValue),
    /// Push a value to a vector
    VectorAppend(StorageKey, StorageValue),
    /// Pop a value from a vector
    VectorPop(StorageKey),
    /// Get the length of a vector
    VectorLength(StorageKey),
    /// Get a value from a map
    MapGet(StorageKey, StorageValue),
    /// Set a value in a map
    MapSet(StorageKey, StorageValue, StorageValue),
    /// Delete a value in a map
    MapDelete(StorageKey, StorageValue),
    /// Get the number of elements in a map
    MapLength(StorageKey),
    /// See if an element exists in a map
    MapExists(StorageKey),
    /// Get the type of some value
    ValueType(StorageKey),
}
