use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread::{self, JoinHandle};

use crate::analysis::{InterpreterRequest, Parser, Tokenizer, Statement};
use crate::auth::AuthorizationLevel;
use crate::error::ServerError;
use crate::io::stream::StreamSender;
use crate::multithreaded::executor::{ExecutorRequest, ExecutorResponse};

pub struct AnalysisRequest {
    request: String,
    authorization: AuthorizationLevel,
    sender: Sender<ExecutorResponse>,
}


/// A worker that takes the raw request and produces statements to be executed.
pub struct AnalysisWorker {
    receive_channel: Arc<Mutex<Receiver<AnalysisRequest>>>,
    send_channel: Sender<ExecutorRequest>,
    shutdown_signal: Arc<AtomicBool>,
    receive_deadline: Duration,
    thread: Option<JoinHandle<()>>,
}


impl AnalysisWorker {
    fn send_response(&mut self, response: ExecutorRequest) {
        let send_result = self.send_channel.send(response);
        if let Err(error) = send_result {
            println!("{:?}", error);
        }
    }

    fn send_error(&mut self, error: ServerError, error_sender: Sender<ExecutorResponse>) {
        let response = ExecutorResponse{response: Err(error)};
        let send_result = error_sender.send(response);
        if let Err(error) = send_result {
            println!("{:?}", error);
        }
    }

    fn process_request(&mut self, request: &str) -> Result<Vec<Statement>, ServerError> {
        let mut tokenizer = Tokenizer::new(&request);
        let tokens = tokenizer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()        
    }

    fn analyze_request(&mut self, request: AnalysisRequest) {
        let AnalysisRequest{request, authorization, sender} = request;
        let request_string = match request {
            Ok(request_string) => request_string,
            Err(error) => {
                self.send_error(error, sender);
                return;
            }
        };
        let statements = self.process_request(&request_string);
        match statements {
            Ok(statements) => {
                let interpreter_request = InterpreterRequest{statements, authorization};
                let exec_request = ExecutorRequest{request: interpreter_request, sender};
                self.send_response(exec_request);
            },
            Err(error) => {
                self.send_error(error, sender);
            }
        }

    }

    /// Search for requests to be processed until ordered to shut down.
    pub fn run(&mut self) {
        loop {
            if self.check_for_shutdown() {
                break;
            }
            let request = match self.receive_channel.try_lock() {
                Ok(ref mut receiver) => {
                    match (**receiver).recv_timeout(self.receive_deadline) {
                       Ok(request) => request,
                       Err(_) => {
                           continue;
                       },
                    }
                }
                Err(_) => {
                    continue;
                }
            };
            self.analyze_request(request);
        } 
    }

    /// Check for a shutdown signal
    fn check_for_shutdown(&mut self) -> bool {
        if self.shutdown_signal.load(Ordering::Relaxed) {
            println!("Shutting down analysis worker.");
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
