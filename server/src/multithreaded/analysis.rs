use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::analysis::{InterpreterRequest, Parser, Tokenizer, Statement};
use crate::auth::AuthorizationLevel;
use crate::error::ServerError;
use crate::io::stream::StreamSender;
use crate::multithreaded::executor::{ExecutorRequest, ExecutorResponse};

struct AnalysisRequest {
    request: Result<String, ServerError>,
    authorization: AuthorizationLevel,
    stream_sender: Option<Box<dyn StreamSender + Send>>,
}


/// A worker that takes the raw request and produces statements to be executed.
pub struct AnalysisWorker {
    receive_channel: Arc<Mutex<Receiver<AnalysisRequest>>>,
    send_channel: Sender<ExecutorRequest>,
    error_channel: Sender<ExecutorResponse>,
    shutdown_signal: Arc<AtomicBool>,
    receive_deadline: Duration,
}


impl AnalysisWorker {
    fn send_response(&mut self, response: ExecutorRequest) {
        let send_result = self.send_channel.send(response);
        if let Err(error) = send_result {
            println!("{:?}", error);
        }
    }

    fn send_error(&mut self, error: ServerError, stream_sender: Option<Box<dyn StreamSender + Send>>) {
        let response = ExecutorResponse{response: Err(error), stream_sender};
        let send_result = self.error_channel.send(response);
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
        let AnalysisRequest{request, authorization, stream_sender} = request;
        let request_string = match request {
            Ok(request_string) => request_string,
            Err(error) => {
                self.send_error(error, stream_sender);
                return;
            }
        };
        let statements = self.process_request(&request_string);
        match statements {
            Ok(statements) => {
                let interpreter_request = InterpreterRequest{statements, authorization};
                let exec_request = ExecutorRequest{request: interpreter_request, stream_sender};
                self.send_response(exec_request);
            },
            Err(error) => {
                self.send_error(error, stream_sender);
            }
        }

    }

    /// Search for requests to be processed until ordered to shut down.
    pub fn run(&mut self) {
        loop {
            let request = match self.receive_channel.try_lock() {
                Ok(ref mut receiver) => {
                    match (**receiver).recv_deadline(Instant::now() + self.receive_deadline) {
                       Ok(request) => request,
                       Err(_) => {
                            if self.check_for_shutdown() {
                                break;
                            }
                           continue;
                       },
                    }
                }
                Err(_) => {
                    if self.check_for_shutdown() {
                        break;
                    }
                    continue;
                }
            };
            if self.check_for_shutdown() {
                break;
            } else {
                self.analyze_request(request);
            }
        } 
    }

    /// Check for a shutdown signal
    fn check_for_shutdown(&mut self) -> bool {
        if self.shutdown_signal.load(Ordering::Relaxed) {
            println!("Shutting down expiration worker.");
            true
        } else {
            false
        }
    }
}