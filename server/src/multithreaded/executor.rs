use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::analysis::{Interpreter, InterpreterRequest, InterpreterResponse};
use crate::error::ServerError;
use crate::storage::hashmap_storage::HashMapStorage;


/// A request to send to the executor
pub struct ExecutorRequest {
    /// The interpreter request
    pub request: InterpreterRequest,
    /// The channel to send a response
    pub sender: Option<Sender<ExecutorResponse>>,
}

/// The response to send back to the requesting channel
pub struct ExecutorResponse {
    /// The result from the interpreter
    pub response: Result<InterpreterResponse, ServerError>,
}

/// An executor sends requests to the interpreter from an open channel and returns responses.
pub struct Executor{
    /// The interpreter backed by some storage object.
    interpreter: Arc<Mutex<Interpreter<HashMapStorage>>>,
    /// The channel handling all requests - many sender/single receiver
    request_channel: Arc<Mutex<Receiver<ExecutorRequest>>>,
    /// A flag to set to shut down all workers prior to shutting down the executor
    start_shutdown_flag: Arc<AtomicBool>,
    /// A flag set to shut down the executor for clean shutdown
    shutdown_flag: Arc<AtomicBool>,
    /// Timeout for receiving a result
    timeout: Duration,
    /// The thread handle
    thread: Option<JoinHandle<()>>
}

impl Executor {
    /// Create a new executor
    pub fn new(request_channel: Receiver<ExecutorRequest>, start_shutdown_flag: Arc<AtomicBool>) -> Executor {
        Executor {
            interpreter: Arc::new(Mutex::new(Interpreter::new(HashMapStorage::new()))),
            request_channel: Arc::new(Mutex::new(request_channel)),
            start_shutdown_flag,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            timeout: Duration::from_secs(1),
            thread: None,
        }

    }

    /// Execute a request
    fn execute(&mut self, request: ExecutorRequest) -> bool {
        let ExecutorRequest{request, sender} = request;
        let interpreter_response = self.interpreter.try_lock().unwrap().interpret(request);
        let keep_going = match interpreter_response {
            Ok(InterpreterResponse::ShuttingDown) => false,
            _ => true,
        };
        let executor_response = ExecutorResponse{response: interpreter_response};
        if let Some(sender) = sender {
            match sender.send(executor_response) {
                Ok(()) => (),
                Err(err) => println!("Error sending response back to listener: {:?}", err),
            }
        }
        keep_going
    }

    /// Loop until told to shut down.
    pub fn run(&mut self) {
        loop {
            if self.shutdown_flag.load(Ordering::Relaxed) {
                println!("Shutting down the executor.");
                break;
            }
            let request = self.request_channel.try_lock().unwrap().recv_timeout(self.timeout);
            let request = match request {
                Ok(request) => request,
                Err(_) => {
                    continue; // A timeout error
                }
            };
            let keep_going = self.execute(request);
            if !keep_going {
                self.start_shutdown_flag.swap(true, Ordering::Relaxed);
            }

        }
    }

    /// Start the worker
    pub fn start(&mut self) {
        println!("Starting executor.");
        let mut temp_worker = Executor{
            interpreter: Arc::clone(&self.interpreter),
            request_channel: Arc::clone(&self.request_channel),
            start_shutdown_flag: Arc::clone(&self.start_shutdown_flag),
            shutdown_flag: Arc::clone(&self.shutdown_flag ),
            timeout: self.timeout.clone(),
            thread: None,
        };
        let join_handle = thread::spawn(move || {
            temp_worker.run();
        });
        self.thread = Some(join_handle);
    }

    /// Stop the worker
    pub fn stop(&mut self) {
        self.shutdown_flag.swap(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            match handle.join() {
                Ok(()) => (),
                Err(err) => {
                    println!("Error stopping the executor. {:?}", err)
                }
            }
        }
    }
}
