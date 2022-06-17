use std::time::Duration;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use tokio::{self, time};
use tokio::sync::mpsc::{self, Sender, Receiver};

use server::auth::{AuthenticationService, MockAuthenticator, AuthorizationLevel, AuthenticationResult};
use server::error::ServerError;
use server::io::tcp_async::{TcpStreamHandler, StreamRequest, TcpStreamSender};
use server::storage::hashmap_storage::HashMapStorage;
use server::analysis::{Interpreter, InterpreterRequest, InterpreterResponse, Parser, Statement, Tokenizer};


const CHANNEL_QUEUE_SIZE: usize = 128;


type ResponseSender = Sender<Result<InterpreterResponse, ServerError>>;
type ExecuteRequest = (InterpreterRequest, Option<ResponseSender>);
type ExecuteSender = Sender<ExecuteRequest>;
type ExecuteReceiver = Receiver<ExecuteRequest>;
type AnalysisRequest = (String, AuthorizationLevel, ResponseSender);
type AnalysisSender = Sender<AnalysisRequest>;
type AnalysisReceiver = Receiver<AnalysisRequest>;


fn authenticate(authenticator: Arc<Mutex<MockAuthenticator>>, headers: &HashMap<String, String>) -> Result<AuthenticationResult, ServerError> {
    let mut authenticator = authenticator.lock().unwrap();
    authenticator.authenticate(&headers)
}


async fn listen_for_requests(analysis_sender: AnalysisSender) {
    let authenticator = Arc::new(Mutex::new(MockAuthenticator));
    let mut stream_handler = TcpStreamHandler::new(IpAddr::V4(Ipv4Addr::new(127, 0,0,1)), 7878).await;
    loop {
        let request = stream_handler.receive_request().await;
        let StreamRequest {request, headers, sender} = request;

        let request = match request {
            Ok(request) => request,
            Err(err) => {
                send_response_to_client(sender, Err(err)).await;
                continue;
            }
        };

        let authentication_result = authenticate(Arc::clone(&authenticator), &headers);
        let (username, authorization)= match authentication_result {
            Ok(AuthenticationResult::Authenticated(username, level)) => (username, level),
            Ok(AuthenticationResult::Unauthenticated) => {
                let err = Err(ServerError::AuthenticationError("Authentication failed.".to_string()));
                send_response_to_client(sender, err).await;
                continue;
            },
            Err(error) => {
                let err = Err(error);
                send_response_to_client(sender, err).await;
                continue;
            },
        };

        let authorization = match authorization {
            None => {
                let error = ServerError::AuthorizationError(
                    format!("User {} not authorized to access this resource.", username)
                );
                let err = Err(error);
                send_response_to_client(sender, err).await;
                continue;
            },
            Some(auth) => auth,
        };
        let (job_sender,  mut job_receiver) = mpsc::channel(1);
        let analysis_request = (request, authorization, job_sender);
        if let Err(err) = analysis_sender.send(analysis_request).await {
            println!("Error sending job to analyzer. {:?}", err);
            send_response_to_client(sender, Err(ServerError::InternalError("Error sending job to analyzer.".to_string()))).await;
            continue;

        }
        let response = job_receiver.recv().await.unwrap();
        send_response_to_client(sender, response).await;


        println!("Listening for requests");
    }
}

async fn send_response_to_client(sender: Option<TcpStreamSender>, response: Result<InterpreterResponse, ServerError>) {
    if let Some(mut sender) = sender {
        if let Err(err) = sender.send(response).await {
            println!("Got error sending message back to client. {:?}", err);
        }
    }
}

async fn send_response(sender: ResponseSender, response: Result<InterpreterResponse, ServerError>) {
    if let Err(err) = sender.send(response).await {
        println!("Error sending result back. {:?}", err);
    }
}

fn process_analyze_request(request: String, authorization: AuthorizationLevel) -> Result<InterpreterRequest, ServerError> {
    let mut tokenizer = Tokenizer::new(&request);
    let tokens = tokenizer.tokenize();
    let tokens = match tokens {
        Ok(tokens) => tokens,
        Err(err) => {
            return Err(err);
        }
    };
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();
    let statements = match statements {
        Ok(statements) => statements,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(InterpreterRequest { statements, authorization })
}


async fn analyze_request(mut analyze_receiver: AnalysisReceiver, execute_sender: ExecuteSender) {
    loop {
        let (request, authorization, sender) = analyze_receiver.recv().await.unwrap();
        let exec_sender = execute_sender.clone();
        tokio::spawn(async move {
            let result = process_analyze_request(request, authorization);
            match result {
                Ok(result) => {
                    let sender_clone = sender.clone();
                    if let Err(err) = exec_sender.send((result, Some(sender))).await {
                        println!("Error sending request to executor {:?}", err);
                        send_response(
                            sender_clone,
                            Err(ServerError::InternalError("Error sending request to executor.".to_string()))
                        ).await;
                    }
                }
                Err(err) => {
                    send_response(sender, Err(err)).await;
                }
            }
        });
    }
}

fn reset_flag(flag: &Arc<Mutex<bool>>, new_value: bool) {
    let mut lock = flag.lock().unwrap();
    *lock = new_value;
}

fn check_flag(flag: &Arc<Mutex<bool>>) -> bool {
    *flag.lock().unwrap()
}

async fn execute_requests(mut receiver: ExecuteReceiver, shutdown_flag: Arc<Mutex<bool>>) {
    let storage = HashMapStorage::new();
    let mut interpreter = Interpreter::new(storage);
    loop {
        let (request, sender) = receiver.recv().await.unwrap();
        let response = interpreter.interpret(request);
        let shutting_down = if let Ok(InterpreterResponse::ShuttingDown) = &response {
            true
        } else {
            false
        };
        if let Some(sender) = sender {
            match sender.send(response).await {
                Ok(_) => (),
                Err(err) => {
                    println!("Error sending response from executor: {:?}", err);
                }
            }
        }
        if shutting_down {
            println!("Received shutdown signal!");
            reset_flag(&shutdown_flag, true);
        }
    }
}

async fn expire_old_keys(execute_sender: ExecuteSender) {
    loop {
        time::sleep(Duration::from_millis(100)).await;
        let request = InterpreterRequest {
            statements: vec![Statement::ExpireKeys], authorization: AuthorizationLevel::Admin
        };
        let sender = None;
        execute_sender.send((request, sender)).await.unwrap();
    }
}


async fn serve() {
    let shutdown_flag = Arc::new(Mutex::new(false));
    let (execute_sender, execute_receiver) = mpsc::channel(CHANNEL_QUEUE_SIZE);
    let (analysis_sender, analysis_receiver) = mpsc::channel(CHANNEL_QUEUE_SIZE);

    let shutdown_copy = Arc::clone(&shutdown_flag);
    tokio::spawn(async move {
        execute_requests(execute_receiver, shutdown_copy).await;
    });

    let execute_sender_analyze = execute_sender.clone();
    tokio::spawn(async move {
        analyze_request(analysis_receiver, execute_sender_analyze).await;
    });
    tokio::spawn(async move {
        expire_old_keys(execute_sender).await;
    });
    tokio::spawn(async move {
        listen_for_requests(analysis_sender).await;
    });
    let mut count = 0;
    async {
        loop {
            count += 1;
            if check_flag(&shutdown_flag) {
                println!("Shutting down.");
                break;
            }
            time::sleep(Duration::from_millis(1000)).await;
        }
    }.await;
    println!("Done.");
}

#[tokio::main]
async fn main() {
    tokio::join!(serve());
}
