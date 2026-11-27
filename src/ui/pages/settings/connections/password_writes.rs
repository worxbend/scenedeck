//! Ordered Secret Service writes for the connection editor's password field.

use std::rc::Rc;

/// The worker snapshot is the last successful operation, not the latest draft.
/// Only the latest submitted request may publish cache/status on GTK. Keeping
/// confirmation on the FIFO worker also handles a successful old set followed
/// by a failed delete whose GTK callbacks happen to arrive in reverse order.
#[derive(Clone)]
pub(super) struct PasswordWrites {
    latest: Rc<std::cell::Cell<u64>>,
    confirmed: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl PasswordWrites {
    pub(super) fn new(confirmed: Option<String>) -> Self {
        Self {
            latest: Rc::new(std::cell::Cell::new(0)),
            confirmed: std::sync::Arc::new(std::sync::Mutex::new(confirmed)),
        }
    }

    pub(super) fn submit<E, Work, Complete>(
        &self,
        password: Option<String>,
        work: Work,
        complete: Complete,
    ) where
        E: Send + 'static,
        Work: FnOnce(&Option<String>) -> Result<(), E> + Send + 'static,
        Complete: FnOnce(Result<(), E>, Option<String>) + 'static,
    {
        let generation = self.latest.get().wrapping_add(1);
        self.latest.set(generation);
        let latest = Rc::clone(&self.latest);
        let confirmed = std::sync::Arc::clone(&self.confirmed);
        crate::ui::background_io::run_serialized(
            crate::ui::background_io::SerialLane::Secret,
            move || {
                let result = work(&password);
                let mut confirmed = crate::infra::sync::lock_or_recover(confirmed.as_ref());
                if result.is_ok() {
                    *confirmed = password;
                }
                (result, confirmed.clone())
            },
            move |(result, confirmed)| {
                if latest.get() == generation {
                    complete(result, confirmed);
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::PasswordWrites;
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex},
        time::Duration,
    };

    #[test]
    fn password_lane_is_fifo_and_only_latest_confirmed_cache_status_publishes() {
        if !crate::ui::background_io::isolated_test_child("ui::pages::settings::connections::password_writes::tests::password_lane_is_fifo_and_only_latest_confirmed_cache_status_publishes") { return; }
        for (second, first_fails, second_fails, expected) in [
            (None, false, false, None),
            (Some("B"), false, false, Some("B")),
            (Some("B"), false, true, Some("A")),
            (None, false, true, Some("A")),
            (Some("B"), true, false, Some("B")),
            (None, true, true, Some("original")),
        ] {
            let store = Arc::new(Mutex::new(Some("original".to_string())));
            let writes = PasswordWrites::new(Some("original".into()));
            let published = Rc::new(RefCell::new(Vec::new()));
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            writes.submit(
                Some("A".into()),
                {
                    let store = store.clone();
                    move |password| {
                        started_tx.send(()).unwrap();
                        release_rx.recv().unwrap();
                        if first_fails {
                            return Err("first failed");
                        }
                        *store.lock().unwrap() = password.clone();
                        Ok(())
                    }
                },
                {
                    let published = published.clone();
                    move |result, confirmed| {
                        published.borrow_mut().push((result.is_ok(), confirmed))
                    }
                },
            );
            started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
            writes.submit(
                second.map(str::to_string),
                {
                    let store = store.clone();
                    move |password| {
                        if second_fails {
                            return Err("second failed");
                        }
                        *store.lock().unwrap() = password.clone();
                        Ok(())
                    }
                },
                {
                    let published = published.clone();
                    move |result, confirmed| {
                        published.borrow_mut().push((result.is_ok(), confirmed))
                    }
                },
            );
            assert_eq!(store.lock().unwrap().as_deref(), Some("original"));
            assert!(
                published.borrow().is_empty(),
                "draft was published before persistence"
            );
            release_tx.send(()).unwrap();
            let main_loop = glib::MainLoop::new(None, false);
            crate::ui::background_io::flush_pending(Duration::from_secs(1), {
                let main_loop = main_loop.clone();
                move |result| {
                    assert_eq!(result, Ok(()));
                    main_loop.quit();
                }
            });
            main_loop.run();
            assert_eq!(store.lock().unwrap().as_deref(), expected);
            assert_eq!(
                *published.borrow(),
                vec![(!second_fails, expected.map(str::to_string))]
            );
        }
    }
}
