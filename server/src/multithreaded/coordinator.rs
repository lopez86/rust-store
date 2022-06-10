use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use super::executor::{Executor, ExecutorResponse};
use super::expiration::ExpirationWorker;
use super::analysis::AnalysisRequest;
use crate::auth::AuthenticationService;
use crate::storage::Storage;
use crate::io::tcp::TcpStreamHandler;
use crate::io::stream::StreamHandler;
use super::{Worker, ThreadPool};


struct Coordinator<A: AuthenticationService, S: Storage + Send> {
    send_channel: Sender<AnalysisRequest>,
    error_channel: Sender<ExecutorResponse>,
    // Needed for this structs thread
    authenticator: A,
    // Workers
    analysis_pool: ThreadPool<AnalysisWorker>,
    responder_pool: ThreadPool<ResponderWorker>,
    listener_pool: ThreadPool<ListenerWorker>,
    executor: Executor,
    expiration: ExpirationWorker,
    // Shutdown flags
    start_shutdown: Arc<AtomicBool>,
}


impl<'a, A: AuthenticationService, S:Storage + Send> Coordinator<A, S> {
    fn new(listener_threads: usize, analysis_threads: usize, responder_threads: usize) -> Coordinator<A, S> {

    }

    fn serve<H: StreamHandler>(&mut self, stream_handler: H) {
        self.responder_pool.spawn();
        self.executor.spawn();
        self.analysis_pool.spawn();
        self.listener_pool.spawn(stream_handler);
        self.expiration.spawn();
        loop {
            thread::sleep(Duration::from_millis(100));
            if self.check_for_shutdown() {
                self.stop();
                break;
            }
        }
    }

    fn stop(&mut self) {
        println!("Stopping the service.");
        self.listener_pool.stop();
        self.analysis_pool.stop();
        self.expiration.stop();
        self.executor.stop();
        self.responder_pool.stop();
        println!("Finished shutting down all workers.");
    }

    fn check_for_shutdown(&self) -> bool {
        self.start_shutdown.load(Ordering::Relaxed)
    }
}