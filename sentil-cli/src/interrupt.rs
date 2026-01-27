use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

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