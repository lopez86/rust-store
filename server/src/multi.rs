use std::net::{IpAddr, Ipv4Addr};
use server::multithreaded::Coordinator;

fn main() {
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port = 7878;
    let mut coordinator = Coordinator::new(3, 3, ip, port);

    coordinator.serve();

}