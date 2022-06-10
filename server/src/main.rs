use std::net::{IpAddr, IpV4Addr};
use crate::single_threaded::SingleThreadedServer;

/// Run a server.
fn main() {
    let server = SingleThreadedServer::new();
    let stream_handler = TcpStreamHandler::new(ip_addr, port);
    server.serve(stream_handler);
}
