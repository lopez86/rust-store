use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::thread;

use crate::workers::executor::ExecutorRequest;
use crate::analysis::{InterpreterRequest, Privileges, Statement};


/// The expiration worker periodically polls the storage to check if there are expired keys.
/// 
/// The storage is responsible for running its own policy for discovering and removing expired keys.
/// This ensures that the command is constantly being sent to be sure that things are being deleted
/// regularly.
/// 
/// Using a special function available only to this worker, we are able to ensure that things remain 
/// consistent - there is still only one thread to handle commands.
pub struct ExpirationWorker {
    /// The queue
    channel: Sender<ExecutorRequest>,
    /// The interpreter to run statements
    ncalls: usize,
    /// Time interval
    interval: Duration,
    /// Kill signal
    shutdown_signal: Arc<AtomicBool>, 
}


impl ExpirationWorker {
    /// Send a series of requests to expire some keys
    fn expire_keys(&self) {
        for _ in 0..self.ncalls {
            let request = ExecutorRequest {
                request: Ok(InterpreterRequest { statement: Statement::ExpireKeys, privileges: Privileges::Admin}),
                stream_sender: None,
            };
            self.channel.send(request).unwrap();
        }
    }

    /// Loop an expiration request at a standard interval until ordered to shut down.
    pub fn run(&mut self) {
        loop  {
            thread::sleep(self.interval);
            if self.shutdown_signal.load(Ordering::Relaxed) {
                println!("Shutting down expiration worker.");
                break;
            }
            self.expire_keys()

        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
}