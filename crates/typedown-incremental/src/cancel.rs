use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

/// Sentinel type used as the panic payload when a query is cancelled
/// Callers should use `Cancelled::catch` to handle it
#[derive(Debug)]
pub struct Cancelled;

impl Cancelled {
  /// Run `f`, returning `Err(Cancelled)` if it was cancelled, `Ok(T)` otherwise
  pub fn catch<T>(f: impl FnOnce() -> T) -> Result<T, Cancelled> {
    match catch_unwind(AssertUnwindSafe(f)) {
      Ok(value) => Ok(value),
      Err(payload) => {
        if payload.downcast_ref::<Cancelled>().is_some() {
          Err(Cancelled)
        } else {
          resume_unwind(payload)
        }
      }
    }
  }
}
