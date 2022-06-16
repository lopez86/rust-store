//! # Server
//! 
//! This crate defines the key-value database server.
//! 
//! 
#![warn(missing_docs)]

/// Backend functionality primarily around 
pub mod storage;
/// Defines the basic analyzer for simple operations
pub mod analysis;
/// Defines the different workers running different tasks
//pub mod multithreaded;
/// Defines IO operations
pub mod io;
/// Defines error types
pub mod error;
/// Single threaded API
pub mod single_threaded;
/// Authorization & Authentication
pub mod auth;
