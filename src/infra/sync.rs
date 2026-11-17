//! One poison policy for ordinary `std` mutexes.
//!
//! A poisoned lock means a thread panicked while holding it. Dropping that
//! error turns the panic into a silent skip (no client, no pending clear).
//! Recover the guard instead. Callers that must not continue after a partial
//! update keep their own `expect` and are listed in `controller/lifecycle.rs`.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Lock `mutex`, or take the inner value when a previous holder panicked.
///
/// Poison is cleared after the log so one panic does not emit an error on
/// every later lock. The value may still be mid-update. Pass `arc.as_ref()`
/// when the mutex sits behind an `Arc`.
#[track_caller]
pub(crate) fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    let caller = std::panic::Location::caller();
    mutex
        .lock()
        .unwrap_or_else(|poisoned| recover(mutex, poisoned, caller))
}

fn recover<'a, T>(
    mutex: &'a Mutex<T>,
    poisoned: PoisonError<MutexGuard<'a, T>>,
    caller: &'static std::panic::Location<'static>,
) -> MutexGuard<'a, T> {
    tracing::error!(%caller, "recovered a poisoned mutex");
    mutex.clear_poison();
    poisoned.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn recovers_the_value_from_a_poisoned_mutex() {
        let mutex = Mutex::new(7);
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let _guard = mutex.lock().expect("lock before poison");
            panic!("poison the mutex");
        }));
        assert!(panicked.is_err());
        assert!(mutex.is_poisoned());

        {
            let guard = lock_or_recover(&mutex);
            assert_eq!(*guard, 7);
        }
        assert!(!mutex.is_poisoned());
        assert_eq!(*lock_or_recover(&mutex), 7);
    }
}
