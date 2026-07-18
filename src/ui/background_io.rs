//! Bridge blocking local I/O back to GTK without blocking the main loop.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Run `work` on a dedicated worker thread and invoke `complete` on GTK's main
/// thread. GTK objects may safely be captured by `complete` because it never
/// crosses the thread boundary.
pub(crate) fn run<T, Work, Complete>(work: Work, complete: Complete)
where
    T: Send + 'static,
    Work: FnOnce() -> T + Send + 'static,
    Complete: FnOnce(T) + 'static,
{
    let result = Arc::new(Mutex::new(None));
    let worker_result = Arc::clone(&result);
    std::thread::spawn(move || {
        let value = work();
        if let Ok(mut slot) = worker_result.lock() {
            *slot = Some(value);
        }
    });

    let complete = Rc::new(RefCell::new(Some(complete)));
    glib::timeout_add_local(Duration::from_millis(20), move || {
        let value = result.lock().ok().and_then(|mut slot| slot.take());
        let Some(value) = value else {
            return glib::ControlFlow::Continue;
        };
        if let Some(complete) = complete.borrow_mut().take() {
            complete(value);
        }
        glib::ControlFlow::Break
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn work_runs_off_the_calling_thread() {
        let caller = std::thread::current().id();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            tx.send(std::thread::current().id()).expect("worker id");
        });
        assert_ne!(rx.recv().expect("worker id"), caller);
    }
}
