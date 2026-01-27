use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();

pub fn flag() -> Arc<AtomicBool> {
    FLAG.get_or_init(|| {
        let flag = Arc::new(AtomicBool::new(false));
        let handler = flag.clone();
        let _ = ctrlc::set_handler(move || handler.store(true, Ordering::SeqCst));
        flag
    })
    .clone()
}

/// Runs a blocking computation while watching for Ctrl+C
pub fn run_or_interrupt<T, F>(work: F) -> Option<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let flag = flag();
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    loop {
        if flag.load(Ordering::SeqCst) {
            return None;
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(value) => return Some(value),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => match worker.join() {
                Err(panic) => std::panic::resume_unwind(panic),
                Ok(()) => return None,
            },
        }
    }
}