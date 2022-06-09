use std::net::{IpAddr, IpV4Addr};
use crate::single_threaded::SingleThreadedServer;

/// Run a server.
fn main() {

    let ip_addr = IpAddr::V4(IpV4Addr::new(127, 0, 0, 1));
    let server = SingleThreadedServer{ip_addr, 7878};
    server.serve();
}
