use std::collections::HashMap;

use crate::auth::{AuthenticationResult, AuthenticationService, MockAuthenticator};
use crate::error::ServerError;
use crate::io::stream::{StreamHandler, StreamRequest};
use crate::analysis::{Interpreter, InterpreterRequest, InterpreterResponse, Parser, Statement, Tokenizer};
use crate::storage::hashmap_storage::HashMapStorage;
use crate::storage::Storage;

/// A server to run everything in a single thread with no async - just loops and runs
pub struct SingleThreadedServer<Auth, Stor>
    where
        Auth: AuthenticationService,
        Stor: Storage + Send,
{
    interpreter: Interpreter<Stor>,
    authenticator: Auth,
}


impl<Auth: AuthenticationService, Stor: Storage + Send> SingleThreadedServer<Auth, Stor> {
    /// Start running the server.
    pub fn serve<H: StreamHandler>(&mut self, mut stream_handler: H) {
        loop {
            println!("Ready to receive request.");
            let request = stream_handler.receive_request();
            if let None = request {
                println!("Stream has closed. Shutting down.");
                break;
            }
            let request: StreamRequest = request.unwrap();
            let StreamRequest{request, sender, headers} = request;
            let (response, shut_down) = self.handle_request(request, headers);
            if let Some(mut sender) = sender{
                let res = sender.send(response);
                match res {
                    Ok(_) => (),
                    Err(error) => println!("{:?}", error),
                }
            }
            if shut_down == true {
                break;
            }
        }
        println!("Shutting down now!");
    }
    
    /// Handle a single stream request to the server. 
    fn handle_request(&mut self, request: Result<String, ServerError>, headers: HashMap<String, String>) -> (Result<InterpreterResponse, ServerError>, bool) {
        println!("Handling request");
        println!("Headers {:?}", headers);
        let authentication = self.authenticator.authenticate(&headers);
        println!("Done with authentication. {:?}", authentication);
        let (username, authorization)= match authentication {
            Ok(AuthenticationResult::Authenticated(username, level)) => (username, level),
            Ok(AuthenticationResult::Unauthenticated) => {
                return (Err(ServerError::AuthenticationError("Authentication failed.".to_string())), false);
            },
            Err(error) => {
                return (Err(error), false);
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
        if let Err(error) = &request {
            return (Err(error.clone()), false);
        }
        let request_string = request.unwrap();

        let mut tokenizer = Tokenizer::new(&request_string);
        let tokens = tokenizer.tokenize();
        if let Err(error) = tokens {
            return (Err(error), false);
        }
        let tokens = tokens.unwrap();
        let mut parser = Parser::new(tokens);
        let statements = parser.parse();
        if let Err(error) = statements {
            return (Err(error), false)
        }
        let statements = statements.unwrap();
        let mut shut_down = false;
        for statement in statements.iter() {
            if let Statement::Shutdown = statement {
                shut_down = true;
                break;
            }
        }
        let int_request = InterpreterRequest{statements, authorization};
        let result = self.interpreter.interpret(int_request);
        (result, shut_down)
    }
}

impl SingleThreadedServer<MockAuthenticator, HashMapStorage> {
    /// Create a new server with our current standard 
    pub fn new() -> SingleThreadedServer<MockAuthenticator, HashMapStorage>  {
        let storage = HashMapStorage::new();
        let authenticator = MockAuthenticator;
        let interpreter = Interpreter{storage};
        SingleThreadedServer{interpreter, authenticator}
    }
}
