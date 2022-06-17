use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread::{self, JoinHandle};

use crate::auth::{AuthenticationService, AuthenticationResult};
use crate::error::ServerError;
use crate::io::stream::{StreamHandler, StreamSender};
use crate::analysis::InterpreterResponse;
use crate::multithreaded::executor::ExecutorResponse;
use crate::multithreaded::analysis::AnalysisRequest;


/// A worker to listen for TCP connections and send off requests to the analyzer.
pub struct ListenerWorker<T: StreamHandler + Send + 'static, A: AuthenticationService + Send + 'static> {
    receive_channel: Arc<Mutex<T>>,
    receive_timeout: Duration,
    send_channel: Sender<AnalysisRequest>,
    shutdown_signal: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    authenticator: Arc<Mutex<A>>,
}


fn send_response(response: Result<InterpreterResponse, ServerError>, sender: Option<Box<dyn StreamSender + Send>>) {
    if let Some(mut sender) = sender {
        let response = sender.send(response);
        if let Err(err) = response {
            println!("{:?}", err);
        }
    }
}

impl<T: StreamHandler + Send + 'static, A: AuthenticationService + Send + 'static> ListenerWorker<T, A> {
    /// Run the worker job.
    fn run(&mut self) {
        loop {
            if self.check_for_shutdown() {
                break;
            }
            let result = {
                let mut lock = self.receive_channel.try_lock();
                let handler = match lock {
                    Ok(ref mut handler) => handler,
                    Err(_) => continue,
                };
                handler.receive_request()
            };

            let request = match result {
                Some(request) => request,
                None => continue,
            };
            let request_string = match request.request {
                Ok(req) => req,
                Err(err) => {
                    send_response(Err(err), request.sender);
                    continue;
                },
            };
            let (
                analysis_request, response_channel
            ) = match self.convert_to_analysis_request(&request_string, &request.headers) {
                Ok((analysis_request, response_channel)) => (analysis_request, response_channel),
                Err(err) => {
                    send_response(Err(err), request.sender);
                    continue;
                }
            };
            match self.send_channel.send(analysis_request) {
                Ok(_) => (),
                Err(err) => {
                    println!("Error sending analysis request: {:?}", err);
                    send_response(Err(ServerError::InternalError("Internal error found.".to_string())), request.sender);
                    continue;
                }
            }
            let response = response_channel.recv_timeout(self.receive_timeout);
            let response = match response {
                Ok(resp) => resp.response,
                Err(_) => Err(ServerError::InternalError("Command timed out.".to_string())),
            };
            send_response(response, request.sender);
        }
    }

    fn convert_to_analysis_request(
        &mut self, request: &str, headers: &HashMap<String, String>
    ) -> Result<(AnalysisRequest, Receiver<ExecutorResponse>), ServerError> {
        let authentication = {
            let mut authenticator = self.authenticator.lock().unwrap();
            authenticator.authenticate(headers)
        };
        let (username, authorization)= match authentication {
            Ok(AuthenticationResult::Authenticated(username, level)) => (username, level),
            Ok(AuthenticationResult::Unauthenticated) => {
                return Err(ServerError::AuthenticationError("Authentication failed.".to_string()));
            },
            Err(error) => {
                return Err(error);
            },
        };

        let authorization = match authorization {
            None => {
                let error = ServerError::AuthorizationError(
                    format!("User {} not authorized to access this resource.", username)
                );
                return Err(error);
            },
            Some(auth) => auth,
        };
        let (sender, receiver) = mpsc::channel();
        let request = AnalysisRequest{request: request.to_string(), authorization, sender: Some(sender)};
        Ok((request, receiver))
    }
    
    /// Check for a shutdown signal
    fn check_for_shutdown(&mut self) -> bool {
        if self.shutdown_signal.load(Ordering::Relaxed) {
            println!("Shutting down listener worker.");
            true
        } else {
            false
        }
    }

    /// Start the worker
    pub fn start(&mut self) {
        let mut temp_worker: ListenerWorker<T, A> = ListenerWorker {
            receive_channel: Arc::clone(&self.receive_channel),
            receive_timeout: self.receive_timeout.clone(),
            send_channel: self.send_channel.clone(),
            shutdown_signal: Arc::clone(&self.shutdown_signal),
            thread: None,
            authenticator: Arc::clone(&self.authenticator),
        };

        self.thread = Some(thread::spawn(move || {
            temp_worker.run()
        }));
    }

    /// Stop the worker
    pub fn stop(&mut self) {
        self.shutdown_signal.swap(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            match handle.join() {
                Ok(()) => (), 
                Err(err) => println!("Error stopping listener worker: {:?}", err),
            }
        }
    }
}

/// A pool of Listener workers for scaling
pub struct ListenerPool<H: StreamHandler + Send + 'static, A: AuthenticationService + Send + 'static> {
    workers: Vec<ListenerWorker<H, A>>,
    shutdown_signal: Arc<AtomicBool>,
}

impl<H: StreamHandler + Send + 'static, A: AuthenticationService + Send + 'static> ListenerPool<H, A> {
    /// Create a new pool of Listeners
    pub fn new(
        workers: usize,
        send_channel: Sender<AnalysisRequest>,
        receive_channel: Arc<Mutex<H>>,
        authentication_server: Arc<Mutex<A>>,
    ) -> ListenerPool<H, A> {
        let mut pool = ListenerPool { workers: vec![], shutdown_signal: Arc::new(AtomicBool::new(false)) };
        let receive_timeout = Duration::from_secs(1);
        for _ in 0..workers {
            pool.workers.push(
                ListenerWorker {
                    receive_channel: Arc::clone(&receive_channel),
                    send_channel: send_channel.clone(),
                    shutdown_signal: pool.shutdown_signal.clone(),
                    receive_timeout,
                    thread: None,
                    authenticator: Arc::clone(&authentication_server),
                }
            );
        }
        pool
    }

    /// Start the pool
    pub fn start(&mut self) {
        for worker in self.workers.iter_mut() {
            worker.start();
        }
    }

    /// Stop the pool
    pub fn stop(&mut self) {
        self.shutdown_signal.swap(true, Ordering::Relaxed);
        for worker in self.workers.iter_mut() {
            worker.stop();
        }

    }
}
