use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use super::executor::ExecutorResponse;
use super::analysis::AnalysisRequest;



struct Coordinator {
    send_channel: Sender<AnalysisRequest>,
    error_channel: Sender<ExecutorResponse>,

}


impl Coordinator {
    fn new(listener_threads: usize, analysis_threads: usize, responder_threads: usize) -> Coordinator {

    }
}