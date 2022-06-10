use crate::auth::{AuthenticationResult, AuthenticationService, MockAuthenticator};
use crate::error::ServerError;
use crate::io::stream::{StreamHandler, StreamRequest};
use crate::analysis::{Interpreter, InterpreterRequest, InterpreterResponse, Parser, Statement, Tokenizer};
use crate::storage::hashmap_storage::HashMapStorage;
use crate::storage::Storage;

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
    pub fn serve<H: StreamHandler>(&mut self, stream_handler: H) {
        while let request = stream_handler.receive_request() {
            if let None = request {
                println!("Stream has closed. Shutting down.");
                break;
            }
            let request: StreamRequest = request.unwrap();
            let (response, shut_down) = self.handle_request(&request);
            if let Some(sender) = request.sender {
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
    fn handle_request(&mut self, request: &StreamRequest) -> (Result<InterpreterResponse, ServerError>, bool) {
        let authentication = self.authenticator.authenticate(&(request.headers));
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
        if let Err(error) = request.request {
            return (Err(error), false);
        }
        let request_string = &request.request.unwrap();

        let tokenizer = Tokenizer::new(&request_string);
        let tokens = tokenizer.tokenize();
        if let Err(error) = tokens {
            return (Err(error), false);
        }
        let tokens = tokens.unwrap();
        let parser = Parser::new(tokens);
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
