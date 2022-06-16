use std::net::{IpAddr, TcpStream, TcpListener};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::iter::Extend;

use httparse::{self, Request, Status};
use serde_json::{self, Value};

use crate::analysis::InterpreterResponse;
use crate::error::{self, ServerError};
use crate::io::stream::{StreamHandler, StreamRequest, StreamSender};


const MAX_BUFFER_SIZE: usize = 1024;
const MAX_NUMBER_OF_HEADERS: usize = 32;


/// Object to send responses back through a TCP stream object.
pub struct TcpStreamSender {
    stream: TcpStream
}


/// Create a properly formatted HTTP response
fn make_response(code: &str, json_payload: &str) -> String {
    format!("HTTP/1.1 {}\n\
    Connection: Closed\n\
    Content-Type: application/json\n\
    Content-Length: {}\n\
     \n\
    {}\n", code, json_payload.len(), json_payload)
}


impl StreamSender for TcpStreamSender {
    fn send(&mut self, response: Result<InterpreterResponse, ServerError>) -> Result<(), ServerError> {
        let (code, json_payload) = match response {
            Ok(response) => {
                let code = "200 Ok".to_string();
                let payload = serde_json::json!(response).to_string();
                (code, payload)
            },
            Err(error) => {
                let code = error::get_error_code(&error);
                (code, format!("{}", error))

            }
        };

        let http_response = make_response(&code, &json_payload);
        let http_bytes = http_response.as_bytes();

        if let Err(_) = self.stream.write(http_bytes) {
            return Err(ServerError::NetworkError("Error writing to stream.".to_string()));
        };
        if let Err(_) = self.stream.flush() {
            return Err(ServerError::NetworkError("Error flushing write buffer for stream.".to_string()));
        }
        Ok(())
    } 
}


/// Extract the length of the query body from an HTTP request.
fn extract_body_length_from_request(request: &Request) -> Result<Option<usize>, ServerError> {
    let mut length: Option<usize> = None;
    for header in request.headers.iter() {
        if header.name == "Content-Length" {
            let length_str = match String::from_utf8(header.value.clone().to_vec()) {
                Ok(utf_str) => utf_str,
                Err(_) => return Err(ServerError::RequestError("Malformed request.".to_string())),
            };
            length = match length_str.parse() {
                Ok(value) => Some(value),
                Err(_) => return Err(ServerError::RequestError("Malformed request.".to_string())),
            };
            break;
        }
    }
    Ok(length)
}

/// Extract the actual request/query string from the body in the POST request.
fn extract_request_from_body(body: &str) -> Result<String, ServerError> {
    let json_value: Result<Value, _> = serde_json::from_str(&body);
    let map = match json_value {
        Ok(Value::Object(map)) => map,
        _ => return Err(ServerError::RequestError("Malformed request.".to_string())),
    };
    let query = match map.get("query") {
        Some(Value::String(query)) => query,
        _ => return Err(ServerError::RequestError("Malformed request.".to_string())),
    };
    Ok(query.clone())
}


/// Convert the headers of an HTTP request into a hashmap.
fn convert_headers_to_map(request: &Request) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for header in request.headers.iter() {
        if let Ok(value) = String::from_utf8(header.value.to_vec()) {
            map.insert(header.name.to_string(), value);
        }
    }
    map
}


/// Convert the stream input into a request object
fn convert_stream_to_request(mut stream: TcpStream) -> StreamRequest {
    let mut buffer = vec![];
    let mut headers = HashMap::new();
    let (headers, body) = loop {
        let mut temp_buffer = [0; MAX_BUFFER_SIZE];
        let mut headers_list = [httparse::EMPTY_HEADER; MAX_NUMBER_OF_HEADERS];
        let mut request = Request::new(&mut headers_list);
        let size_read = match stream.read(&mut temp_buffer) {
            Ok(read) => read,
            Err(_) => {
                let sender: Option<Box<dyn StreamSender + Send>> = Some(Box::new(TcpStreamSender{stream}));
                return StreamRequest {
                    request: Err(ServerError::NetworkError("Problem reading request.".to_string())),
                    headers,
                    sender,
                };
            },
        };
        buffer.extend(&temp_buffer[..size_read]);
        let body_start = match request.parse(&buffer) {
            Ok(Status::Complete(size)) => size,
            _ => return StreamRequest {
                request: Err(ServerError::NetworkError("Problem reading request.".to_string())),
                headers,
                sender: Some(Box::new(TcpStreamSender{stream})),
            },
        };
        let body_length = match extract_body_length_from_request(&request) {
            Ok(length) => length,
            Err(err) => {
                return StreamRequest {
                    request: Err(err),
                    headers,
                    sender: Some(Box::new(TcpStreamSender{stream})),
                };
            },
        };
        let body_length = match body_length {
            None => continue,
            Some(body_length) => body_length,
        };
        if (body_start as u64 + body_length as u64) as usize <= buffer.len() {
            match request.method {
                Some("POST") => (),
                _ => return StreamRequest {
                    request: Err(ServerError::RequestError("Malformed request.".to_string())),
                    headers,
                    sender: Some(Box::new(TcpStreamSender{stream})),
                }
            }
            headers = convert_headers_to_map(&request);
            let body = String::from_utf8_lossy(&buffer[body_start..(body_start + body_length)]);

            break (headers, body);
        }
    };

    let query = match extract_request_from_body(&body) {
        Ok(query) => query,
        Err(err) => {
            return StreamRequest {
                request: Err(err),
                headers,
                sender: Some(Box::new(TcpStreamSender{stream})),
            };
        }
    };
    let request = Ok(query);
    StreamRequest { request, headers, sender: Some(Box::new(TcpStreamSender{stream}))}
}


/// Handles connections from a TCP listener.
pub struct TcpStreamHandler {
    listener: TcpListener,
}


impl TcpStreamHandler {
    /// Create a new TCP connection bound to an IP address and a port.
    pub fn new(ip_address: IpAddr, port: usize) -> TcpStreamHandler {
        let listener = TcpListener::bind(format!("{}:{}", ip_address.to_string(), port)).unwrap();
        //let incoming = listener.incoming();
        TcpStreamHandler{listener}
    }
}


impl StreamHandler for TcpStreamHandler {
    fn receive_request(&mut self) -> Option<StreamRequest> {
        let stream = self.listener.accept();
        let stream = match stream {
            Err(_) => return Some(
                StreamRequest {
                    request: Err(ServerError::NetworkError("Could not read TCP connection.".to_string())),
                    headers: HashMap::new(),
                    sender: None,
                }
            ),
            Ok((stream, _)) => stream,
        };
        Some(convert_stream_to_request(stream))
    }
}
