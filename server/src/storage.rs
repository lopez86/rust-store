/// Contains generic types for backend storage
mod types;

pub use self::types::{*};

/// Contains an implementation of storage using a HashMap
pub mod hashmap_storage;
