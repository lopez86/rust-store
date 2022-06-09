use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::thread::{self, JoinHandle};

use crate::multithreaded::executor::ExecutorRequest;
use crate::analysis::{InterpreterRequest, Statement};
use crate::auth::AuthorizationLevel;


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
    /// Thread
    thread: Option<JoinHandle<()>>,
}


impl ExpirationWorker {
    /// Send a series of requests to expire some keys
    fn expire_keys(&self) {
        for _ in 0..self.ncalls {
            let request = ExecutorRequest {
                request: InterpreterRequest {
                    statements: vec![Statement::ExpireKeys], authorization: AuthorizationLevel::Admin
                },
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

    /// Spawn a thread
    pub fn spawn(&mut self) {
        let join_handle = thread::spawn(|| {
            self.run();
        });
        self.thread = Some(join_handle);
    }
}



#[cfg(test)]
mod tests {
    use super::*;
}