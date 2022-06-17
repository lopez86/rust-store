use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread::{self, JoinHandle};

use crate::analysis::{InterpreterRequest, Parser, Tokenizer, Statement};
use crate::auth::AuthorizationLevel;
use crate::error::ServerError;
use crate::multithreaded::executor::{ExecutorRequest, ExecutorResponse};

/// Request for an analyzer
pub struct AnalysisRequest {
    /// The request string
    pub request: String,
    /// The authorization level for this request
    pub authorization: AuthorizationLevel,
    /// A sender back to the listener node for responding
    pub sender: Option<Sender<ExecutorResponse>>,
}


/// A worker that takes the raw request and produces statements to be executed.
pub struct AnalysisWorker {
    /// The channel to receive requests
    receive_channel: Arc<Mutex<Receiver<AnalysisRequest>>>,
    /// The channel to send requests to the executor
    send_channel: Sender<ExecutorRequest>,
    /// Flag to manage shutdowns
    shutdown_signal: Arc<AtomicBool>,
    /// Length to wait for receiving before stopping and checking for shutdown
    receive_deadline: Duration,
    /// Thread handle for bookkeeping
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
        let statements = self.process_request(&request);
        match statements {
            Ok(statements) => {
                let interpreter_request = InterpreterRequest{statements, authorization};
                let exec_request = ExecutorRequest{request: interpreter_request, sender};
                self.send_response(exec_request);
            },
            Err(error) => {
                if let Some(sender) = sender {
                    self.send_error(error, sender);
                }
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

    /// Start the worker
    pub fn start(&mut self) {
        println!("Starting analysis worker.");
        let mut temp_worker = AnalysisWorker {
            receive_channel: Arc::clone(&self.receive_channel),
            send_channel: self.send_channel.clone(),
            shutdown_signal: Arc::clone(&self.shutdown_signal),
            receive_deadline: self.receive_deadline.clone(),
            thread: None,
        };
        let join_handle = thread::spawn(move || {
            temp_worker.run();
        });
        self.thread = Some(join_handle);
    }

    /// Stop the worker
    pub fn stop(&mut self) {

        if let Some(handle) = self.thread.take() {
            match handle.join() {
                Ok(()) => (),
                Err(err) => println!("Error stopping thread {:?}.", err),
            }
        }
    }
}

/// Pool of analyzers for scaling
pub struct AnalysisPool {
    workers: Vec<AnalysisWorker>,
    shutdown_signal: Arc<AtomicBool>,
}

impl AnalysisPool {
    /// Create a new pool
    pub fn new(
        workers: usize,
        send_channel: Sender<ExecutorRequest>,
        receive_channel: Arc<Mutex<Receiver<AnalysisRequest>>>
    ) -> AnalysisPool {
        let mut pool = AnalysisPool { workers: vec![], shutdown_signal: Arc::new(AtomicBool::new(false)) };
        let receive_deadline = Duration::from_secs(1);
        for _ in 0..workers {
            pool.workers.push(
                AnalysisWorker {
                    receive_channel: receive_channel.clone(),
                    send_channel: send_channel.clone(),
                    shutdown_signal: pool.shutdown_signal.clone(),
                    receive_deadline,
                    thread: None,
                }
            );
        }
        pool
    }

    /// Start the pool
    pub fn start(&mut self) {
        println!("Starting analysis pool.");
        for worker in self.workers.iter_mut() {
            worker.start();
        }
    }

    /// Stop the pool
    pub fn stop(&mut self) {
        println!("Shutting down analysis pool.");
        self.shutdown_signal.swap(true, Ordering::Relaxed);
        for worker in self.workers.iter_mut() {
            worker.stop();
        }

    }
}
