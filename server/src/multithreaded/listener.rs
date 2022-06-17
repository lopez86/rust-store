use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread::{self, JoinHandle};

use crate::auth::{AuthenticationService, AuthenticationResult};
use crate::error::ServerError;
use crate::io::stream::StreamHandler;
use crate::multithreaded::executor::ExecutorResponse;
use crate::multithreaded::analysis::AnalysisRequest;

pub struct ListenerWorker<T: StreamHandler, A: AuthenticationService> {
    receive_channel: Arc<Mutex<T>>,
    receive_timeout: Duration,
    send_channel: Sender<AnalysisRequest>,
    response_channel: Receiver<ExecutorResponse>,
    shutdown_signal: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    authorization: A,
}

fn send_error<S: StreamHandler>(err: ServerError, sender: Option<S>) {
    let response = ExecutorResponse{response: Err(err)};
    send_response(response, sender);
}

fn send_response<S: StreamHandler>(response: ExecutorResponse, sender: Option<S>) {
    if let Some(sender) = sender {
        let response = sender.send(response);
        if let Err(err) = response {
            println!("{:?}", err);
        }
    }
}

impl<T: StreamHandler, A: AuthenticationService> ListenerWorker<T, A> {
    //fn handle_connection(&mut self, connection: )

    fn run(&mut self) {
        loop {
            if self.check_for_shutdown() {
                break;
            }
            let result = {
                let handler = match self.receive_channel.try_lock() {
                    Ok(ref mut handler) => handler,
                    Err(_) => continue,
                };
                handler.receive_request()
            };

            let request = match result {
                Some(request) => request,
                None => continue,
            };
            let (
                analysis_request, response_channel
            ) = match self.convert_to_analysis_request(request.request, &request.headers) {
                Ok((analysis_request, response_channel)) => (analysis_request, response_channel),
                Err(err) => {
                    send_error(err, request.sender);
                    continue;
                }
            };
            self.send_channel.send(analysis_request);
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
        let authentication = self.authenticator.authenticate(headers);
        let (username, authorization)= match authentication {
            Ok(AuthenticationResult::Authenticated(username, level)) => (username, level),
            Ok(AuthenticationResult::Unauthenticated) => {
                return (Err(ServerError::AuthenticationError("Authentication failed.".to_string())), false);
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
                return (Err(error), false);
            },
            Some(auth) => auth,
        };
        let (sender, receiver) = mpsc::channel();
        let request = AnalysisRequest{request: request.to_string(), authorization, sender};
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

    fn spawn(&mut self) {
        self.thread = Some(thread::spawn(|| {
            self.run()
        }));
    }
}

