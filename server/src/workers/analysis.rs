use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::analysis::{InterpreterRequest, Parser, Tokenizer, Statement};
use crate::error::ServerError;
use crate::io::stream::StreamRequest;
use crate::workers::executor::ExecutorRequest;


/// A worker that takes the raw request and produces statements to be executed.
pub struct AnalysisWorker {
    receive_channel: Receiver<StreamRequest>,
    send_channel: Sender<ExecutorRequest>,
    shutdown_signal: Arc<AtomicBool>,
}

impl AnalysisWorker {
    fn send_response(&mut self, response: ExecutorRequest) {
        let send_result = self.send_channel.send(response);
        if let Err(error) = send_result {
            println!("{:?}", error);
        }
    }

    fn analyze_request(&mut self, request: StreamRequest) {
        let StreamRequest{request, privileges, sender} = request;
        let request_string = match request {
            Ok(request_string) => request_string,
            Err(error) => {
                let ex_request = ExecutorRequest {
                    request: Err(error), stream_sender: Some(sender)
                };
                self.send_response(ex_request);
                return;
            }
        };
        let mut tokenizer = Tokenizer::new(&request_string);
        let tokens = tokenizer.tokenize();
        let tokens = match tokens {
            Ok(tokens) => tokens,
            Err(error) => {
                let ex_request = ExecutorRequest {
                    request: Err(error), stream_sender: Some(sender)
                };
                self.send_response(ex_request);
                return
            },
        };
        let mut parser = Parser::new(tokens);
        let parse_result = parser.parse();
        if let Err(error) = parse_result {
            let ex_request = ExecutorRequest {
                request: Err(error), stream_sender: Some(sender)
            };
            self.send_response(ex_request);
            return;
        }
        let mut statements = parse_result.unwrap();
        let interpreter_request = match statements.len() {
            0 => Ok(InterpreterRequest{statement: Statement::Null, privileges}),
            1 => Ok(InterpreterRequest{statement: statements.pop().unwrap(), privileges}),
            _ => Err(ServerError::ParseError("Multiple statements found in request.".to_string())),
        };
        let ex_request = ExecutorRequest {
            request: interpreter_request, stream_sender: Some(sender)
        };
        self.send_response(ex_request);
        
    }

    /// Search for requests to be processed until ordered to shut down.
    pub fn run(&mut self) {
        for request in self.receive_channel.recv() {
            if self.shutdown_signal.load(Ordering::Relaxed) {
                println!("Shutting down expiration worker.");
                break;
            }
            self.analyze_request(request);
        }

    }

}