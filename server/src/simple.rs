use std::net::{IpAddr, Ipv4Addr};
use server::io::tcp::TcpStreamHandler;
use server::single_threaded::SingleThreadedServer;

/// Run a server.
fn main() {
    let mut server = SingleThreadedServer::new();
    let stream_handler = TcpStreamHandler::new(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), 7878);
    server.serve(stream_handler);
}
