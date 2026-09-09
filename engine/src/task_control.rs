use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Cooperative cancellation shared by a view and one background operation.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err("Operation cancelled.".into())
        } else {
            Ok(())
        }
    }
}
