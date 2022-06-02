use std::fmt::Debug;

use client::data_types::StorageKey;

use crate::storage::StorageValue;

/// Lifetime in seconds of a 
type Lifetime = u64;


/// Statement
#[derive(Clone, Debug)]
pub enum Statement {
    Get(StorageKey),
    Set(StorageKey, StorageValue, Option<Lifetime>),
    Update(StorageKey, StorageValue, Option<Lifetime>),
    Exists(StorageKey),
    Delete(StorageKey),
    GetLifetime(StorageKey),
    UpdateLifetime(StorageKey, Option<Lifetime>),
    GetIfExists(StorageKey),
    SetIfNotExists(StorageKey, StorageValue, Option<Lifetime>),
    VectorGet(StorageKey, usize),
    VectorSet(StorageKey, usize, StorageValue),
    VectorAppend(StorageKey, StorageValue),
    VectorPop(StorageKey),
    VectorLength(StorageKey),
    MapGet(StorageKey, StorageValue),
    MapSet(StorageKey, StorageValue, StorageValue),
    MapDelete(StorageKey, StorageValue),
    MapLength(StorageKey),
    MapExists(StorageKey),
    ValueType(StorageKey),
    Null,
}
