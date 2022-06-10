use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread::{self, JoinHandle};

use crate::multithreaded::executor::ExecutorResponse;

pub struct ResponderWorker {
    receive_channel: Arc<Mutex<Receiver<ExecutorResponse>>>,
    receive_timeout: Duration,
    shutdown_signal: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ResponderWorker {
    pub fn run(&mut self) {
        loop {
            if self.check_for_shutdown() {
                break;
            }
            let request = match self.receive_channel.try_lock() {
                Ok(ref mut receiver) => {
                    match (**receiver).recv_timeout(self.receive_timeout) {
                        Ok(request) => request,
                        Err(_) => continue,
                    }
                },
                Err(_) => continue,
            };
            let ExecutorResponse{response, stream_sender} = request;
            if let Some(stream_sender) = stream_sender {
                stream_sender.send(response);
            }
        }
    }

    fn check_for_shutdown(&mut self) -> bool {
        if self.shutdown_signal.load(Ordering::Relaxed) {
            println!("Shutting down responder worker.");
            true
        } else {
            false
        }
    }

    /// Spawn a thread
    pub fn spawn(&mut self) {
        let join_handle = thread::spawn(|| {
            self.run();
        });
        self.thread = Some(join_handle);
    }
    
    pub fn stop(&mut self) {
        unimplemented!("This is not implemented");
    }
}