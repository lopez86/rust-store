use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Analysis workers analyze the query and send resulting statements to the executor
pub mod analysis;
/// The executor runs the statements
pub mod executor;
/// The expiration worker invalidates expiring keys
pub mod expiration;
/// The main coordinating worker
pub mod coordinator;
/// Main endpoint for sending out responses
pub mod responder;


pub trait Worker {

}

pub struct ThreadPool<W: Worker> {
    workers: Vec<W>,
    stop_flag: Arc<AtomicBool>,
}

impl<W: Worker> ThreadPool<W> {

    fn spawn(&mut self) {
        unimplemented!("This is not implemented");
    }

    fn stop(&mut self) {
        unimplemented!("This is not implemented");
    }
}