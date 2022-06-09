use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;

use crate::analysis::{Interpreter, InterpreterRequest, InterpreterResponse};
use crate::error::ServerError;
use crate::storage::Storage;
use crate::io::stream::StreamSender;


/// A request to send to the executor
pub struct ExecutorRequest {
    /// The interpreter request
    pub request: InterpreterRequest,
    /// The channel to send a response
    pub stream_sender: Option<Box<dyn StreamSender + Send>>,
}

/// The response to send back to the requesting channel
pub struct ExecutorResponse {
    /// The result from the interpreter
    pub response: Result<InterpreterResponse, ServerError>,
    /// The object to process and send back responses
    pub stream_sender: Option<Box<dyn StreamSender + Send>>,
}

/// An executor sends requests to the interpreter from an open channel and returns responses.
pub struct Executor<S: Storage> {
    /// The interpreter backed by some storage object.
    interpreter: Interpreter<S>,
    /// The channel handling all requests - many sender/single receiver
    request_channel: Receiver<ExecutorRequest>,
    /// A flag to set to shut down all workers prior to shutting down the executor
    shutdown_workers_flag: Arc<AtomicBool>,
    /// A flag set to shut down the executor for clean shutdown
    shutdown_flag: Arc<AtomicBool>,
}

impl<S: Storage> Executor<S> {
    /// Execute a request
    fn execute(&mut self, request: ExecutorRequest) -> bool {
        let ExecutorRequest{request, stream_sender} = request;
        let interpreter_response = self.interpreter.interpret(request);
        let keep_going =         match interpreter_response {
            Ok(InterpreterResponse::ShuttingDown) => false,
            _ => true,
        };
        let executor_response = ExecutorResponse{response: interpreter_response, stream_sender};
        self.send_response(executor_response);
        keep_going
    }

    /// Send a response to the responder pool for final processing and sending
    fn send_response(&mut self, _response: ExecutorResponse) {
        unimplemented!("This function is not implemented")
    }

    /// Loop until told to shut down.
    pub fn run(&mut self) {
        for request in self.request_channel.recv() {
            let keep_going = self.execute(request);
            if !keep_going {
                self.shutdown_workers_flag.swap(true, Ordering::Relaxed);
                break;
            }
        }
        // Prevents the receiver from being dropped until the flag is set for cleaner
        // shutdown
        loop {
            if self.shutdown_flag.load(Ordering::Relaxed) {
                println!("Shutting down the executor.");
                break;
            }
        }
    }
}
