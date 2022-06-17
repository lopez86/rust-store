use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::analysis::{Interpreter, InterpreterRequest, InterpreterResponse};
use crate::error::ServerError;
use crate::io::stream::StreamSender;
use crate::storage::hashmap_storage::HashMapStorage;


/// A request to send to the executor
pub struct ExecutorRequest {
    /// The interpreter request
    pub request: InterpreterRequest,
    /// The channel to send a response
    pub sender: Sender<ExecutorResponse>,
}

/// The response to send back to the requesting channel
pub struct ExecutorResponse {
    /// The result from the interpreter
    pub response: Result<InterpreterResponse, ServerError>,
}

/// An executor sends requests to the interpreter from an open channel and returns responses.
pub struct Executor{
    /// The interpreter backed by some storage object.
    interpreter: Interpreter<HashMapStorage>,
    /// The channel handling all requests - many sender/single receiver
    request_channel: Receiver<ExecutorRequest>,
    /// A flag to set to shut down all workers prior to shutting down the executor
    start_shutdown_flag: Arc<AtomicBool>,
    /// A flag set to shut down the executor for clean shutdown
    shutdown_flag: Arc<AtomicBool>,
    /// The thread handle
    thread: Option<JoinHandle<()>>
}

impl Executor {
    /// Execute a request
    fn execute(&mut self, request: ExecutorRequest) -> bool {
        let ExecutorRequest{request, sender} = request;
        let interpreter_response = self.interpreter.interpret(request);
        let keep_going = match interpreter_response {
            Ok(InterpreterResponse::ShuttingDown) => false,
            _ => true,
        };
        let executor_response = ExecutorResponse{response: interpreter_response};
        match sender.send(executor_response) {
            Ok(()) => (),
            Err(err) => println!("Error sending response back to listener."),
        }
        keep_going
    }

    /// Loop until told to shut down.
    pub fn run(&mut self) {
        for request in self.request_channel.recv() {
            let keep_going = self.execute(request);
            if !keep_going {
                self.start_shutdown_flag.swap(true, Ordering::Relaxed);
            }
            if self.shutdown_flag.load(Ordering::Relaxed) {
                println!("Shutting down the executor.");
                break;
            }
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
