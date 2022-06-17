use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::net::IpAddr;

use super::executor::Executor;
use super::expiration::ExpirationWorker;
use crate::auth::MockAuthenticator;
use crate::io::tcp::TcpStreamHandler;
use super::listener::ListenerPool;
use super::analysis::AnalysisPool;


/// Higher level struct to run a multithreaded server.
pub struct Coordinator {
    /// Pool of listeners
    listener_pool: ListenerPool<TcpStreamHandler, MockAuthenticator>,
    /// Pool of analyzers
    analysis_pool: AnalysisPool,
    /// Executor worker
    executor: Executor,
    /// Old key expiration worker
    expiration: ExpirationWorker,
    /// Flag to kick off shutdown process
    start_shutdown: Arc<AtomicBool>,
}


impl Coordinator
{
    /// Create a new Coordinator
    pub fn new(listeners: usize, analyzers: usize, ip_addr: IpAddr, port: usize) -> Coordinator {
        let handler = TcpStreamHandler::new(ip_addr, port);
        let handler = Arc::new(Mutex::new(handler));
        let authenticator = Arc::new(Mutex::new(MockAuthenticator));
        let (analysis_send_channel, analysis_receive_channel) = mpsc::channel();
        let analysis_receive_channel = Arc::new(Mutex::new(analysis_receive_channel));
        let (executor_send_channel, executor_receive_channel) = mpsc::channel();

        let listener_pool = ListenerPool::new(
            listeners, analysis_send_channel, handler, authenticator
        );
        let analysis_pool = AnalysisPool::new(
            analyzers,
            executor_send_channel.clone(),
            analysis_receive_channel,
        );

        let start_shutdown = Arc::new(AtomicBool::new(false));
        let executor = Executor::new(executor_receive_channel, Arc::clone(&start_shutdown));

        let expiration = ExpirationWorker::new(executor_send_channel.clone());
    
        Coordinator {
            listener_pool,
            analysis_pool,
            executor,
            expiration,
            start_shutdown
        }
    }

    /// Start the server
    pub fn serve(&mut self) {
        self.executor.start();
        self.analysis_pool.start();
        self.listener_pool.start();
        self.expiration.start();
        println!("Ready for requests.");
        loop {
            thread::sleep(Duration::from_secs(1));
            if self.check_for_shutdown() {
                println!("Shutdown signal received.");
                self.stop();
                break;
            }
        }
    }

    /// Stop the server
    fn stop(&mut self) {
        println!("Stopping the service.");
        self.listener_pool.stop();
        self.analysis_pool.stop();
        self.expiration.stop();
        self.executor.stop();
        println!("Finished shutting down all workers.");
    }

    fn check_for_shutdown(&self) -> bool {
        self.start_shutdown.load(Ordering::Relaxed)
    }
}