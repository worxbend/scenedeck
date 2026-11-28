use std::time::Duration;

fn drain(expected: Result<(), super::FlushError>) {
    let main_loop = glib::MainLoop::new(None, false);
    let outcome = std::rc::Rc::new(std::cell::Cell::new(None));
    super::flush_pending(Duration::from_secs(2), {
        let main_loop = main_loop.clone();
        let outcome = outcome.clone();
        move |result| {
            outcome.set(Some(result));
            main_loop.quit();
        }
    });
    main_loop.run();
    assert_eq!(outcome.get(), Some(expected));
}

#[test]
fn retired_config_batch_cannot_admit_untracked_work_through_worker_arc() {
    if !super::isolated_test_child(
        "ui::background_io::tests::retired_config_batch_cannot_admit_untracked_work_through_worker_arc",
    ) {
        return;
    }
    let (release_first, first_held) = std::sync::mpsc::channel();
    super::run_config_snapshot(move || {
        first_held.recv().unwrap();
        Ok(())
    });
    let (idle, observed_idle) = std::sync::mpsc::channel();
    let (release_worker, worker_held) = std::sync::mpsc::channel();
    super::CONFIG_TAIL.with(|tail| {
        let slot = tail.borrow().upgrade().unwrap();
        slot.lock().unwrap().on_idle = Some(Box::new(move || {
            idle.send(()).unwrap();
            worker_held.recv().unwrap();
        }));
    });
    release_first.send(()).unwrap();
    observed_idle.recv_timeout(Duration::from_secs(2)).unwrap();
    // The actual batch has finished, but its actual worker still owns its
    // Arc. Let the real GTK publication retire the ticket in that window.
    drain(Ok(()));
    super::PENDING.with(|pending| assert_eq!(pending.get(), 0));

    let (started, observed_start) = std::sync::mpsc::channel();
    let (release_second, second_held) = std::sync::mpsc::channel();
    let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    super::run_config_snapshot({
        let completed = completed.clone();
        move || {
            started.send(()).unwrap();
            second_held.recv().unwrap();
            completed.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    });
    super::PENDING
        .with(|pending| assert_eq!(pending.get(), 1, "every admitted batch needs a live ticket"));
    release_worker.send(()).unwrap();
    observed_start.recv_timeout(Duration::from_secs(2)).unwrap();
    let main_loop = glib::MainLoop::new(None, false);
    super::flush_pending(Duration::from_millis(60), {
        let main_loop = main_loop.clone();
        move |result| {
            assert_eq!(result, Err(super::FlushError::TimedOut));
            main_loop.quit();
        }
    });
    main_loop.run();
    assert!(!completed.load(std::sync::atomic::Ordering::SeqCst));
    release_second.send(()).unwrap();
    drain(Ok(()));
    assert!(completed.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn config_publication_keeps_the_newer_tail_and_callback_barrier() {
    if !super::isolated_test_child(
        "ui::background_io::tests::config_publication_keeps_the_newer_tail_and_callback_barrier",
    ) {
        return;
    }
    super::run_config_snapshot(|| Ok(()));
    let (release, held) = std::sync::mpsc::channel();
    let (started, observed_start) = std::sync::mpsc::channel();
    let writes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    super::run_serialized(
        super::SerialLane::Config,
        move || {
            started.send(()).unwrap();
            held.recv().unwrap();
        },
        {
            let writes = writes.clone();
            move |()| {
                // Callback-chained transactions still fence the tail.
                super::run_serialized(
                    super::SerialLane::Config,
                    {
                        let writes = writes.clone();
                        move || writes.lock().unwrap().push(2)
                    },
                    move |()| {
                        super::run_config_snapshot(move || {
                            writes.lock().unwrap().push(3);
                            Ok(())
                        });
                    },
                );
            }
        },
    );
    observed_start.recv_timeout(Duration::from_secs(2)).unwrap();
    super::run_config_snapshot({
        let writes = writes.clone();
        move || {
            writes.lock().unwrap().push(0);
            Ok(())
        }
    });
    // Retire only the completed first batch, while the newer tail is
    // queued behind the blocked transaction and still owns its ticket.
    let context = glib::MainContext::default();
    while super::PENDING.with(|pending| pending.get() == 3) {
        context.iteration(true);
    }
    for _ in 0..128 {
        let writes = writes.clone();
        super::run_config_snapshot(move || {
            writes.lock().unwrap().push(1);
            Ok(())
        });
    }
    super::PENDING.with(|pending| assert_eq!(pending.get(), 2));
    release.send(()).unwrap();
    drain(Ok(()));
    assert_eq!(*writes.lock().unwrap(), [1, 2, 3]);
}

#[test]
fn config_snapshot_error_requires_an_explicit_close_decision() {
    if !super::isolated_test_child(
        "ui::background_io::tests::config_snapshot_error_requires_an_explicit_close_decision",
    ) {
        return;
    }
    super::run_config_snapshot(|| Err(std::io::Error::other("final snapshot failed")));
    drain(Err(super::FlushError::WorkerFailed));
    let decision = super::failure_evidence();

    // Neither a later Config save nor an unrelated lane proves the failed
    // operation was recovered. Cancel/rechecking the drain keeps evidence.
    super::run_config_snapshot(|| Ok(()));
    super::run_serialized(super::SerialLane::Registry, || (), |()| {});
    drain(Err(super::FlushError::WorkerFailed));
    super::run_config_snapshot(|| Err(std::io::Error::other("newer failure")));
    drain(Err(super::FlushError::WorkerFailed));
    super::acknowledge_failures(decision);
    drain(Err(super::FlushError::WorkerFailed));
    super::acknowledge_failures(super::failure_evidence());
    drain(Ok(()));
    super::WORKER_FAILURES.with(|failures| assert_eq!(failures.get(), 2));
}

#[test]
fn config_snapshot_retry_resolves_only_snapshot_failures() {
    if !super::isolated_test_child(
        "ui::background_io::tests::config_snapshot_retry_resolves_only_snapshot_failures",
    ) {
        return;
    }
    assert!(
        !super::retry_config_snapshot(|| panic!("nothing to retry")),
        "a retry without a failed snapshot admits no write"
    );
    super::run_config_snapshot(|| Err(std::io::Error::other("snapshot failed")));
    drain(Err(super::FlushError::WorkerFailed));

    assert!(super::retry_config_snapshot(|| {
        Err(std::io::Error::other("retry failed"))
    }));
    drain(Err(super::FlushError::WorkerFailed));

    let unrelated = super::record_failure();
    assert!(super::retry_config_snapshot(|| Ok(())));
    drain(Err(super::FlushError::WorkerFailed));
    assert!(
        !super::retry_config_snapshot(|| panic!("already resolved")),
        "a successful retry resolves every snapshot failure it covered"
    );
    super::resolve_failure(unrelated);
    drain(Ok(()));
}

#[test]
fn config_snapshot_publication_is_bounded_when_gtk_has_not_polled() {
    if !super::isolated_test_child(
        "ui::background_io::tests::config_snapshot_publication_is_bounded_when_gtk_has_not_polled",
    ) {
        return;
    }
    let (written, observed) = std::sync::mpsc::channel();
    for _ in 0..128 {
        let written = written.clone();
        super::run_config_snapshot(move || {
            written.send(()).unwrap();
            Ok(())
        });
        observed.recv_timeout(Duration::from_secs(2)).unwrap();
    }
    // A fast writer must not turn a busy main context into an unbounded
    // collection of already-complete timers and completion closures.
    super::PENDING.with(|pending| assert_eq!(pending.get(), 1));
    let main_loop = glib::MainLoop::new(None, false);
    super::flush_pending(Duration::from_secs(2), {
        let main_loop = main_loop.clone();
        move |result| {
            assert_eq!(result, Ok(()));
            main_loop.quit();
        }
    });
    main_loop.run();
}

#[test]
fn config_snapshots_are_bounded_and_latest_wins_between_fifo_barriers() {
    if !super::isolated_test_child(
        "ui::background_io::tests::config_snapshots_are_bounded_and_latest_wins_between_fifo_barriers",
    ) {
        return;
    }
    let (release, held) = std::sync::mpsc::channel();
    super::run_serialized(
        super::SerialLane::Config,
        move || held.recv().unwrap(),
        |()| {},
    );
    let writes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for value in 0..1000 {
        let writes = writes.clone();
        super::run_config_snapshot(move || {
            writes.lock().unwrap().push(value);
            Ok(())
        });
    }
    super::PENDING.with(|pending| assert_eq!(pending.get(), 2));
    super::run_serialized(
        super::SerialLane::Config,
        {
            let writes = writes.clone();
            move || writes.lock().unwrap().push(1000)
        },
        |()| {},
    );
    for value in 1001..2000 {
        let writes = writes.clone();
        super::run_config_snapshot(move || {
            writes.lock().unwrap().push(value);
            Ok(())
        });
    }
    // One held writer, two latest snapshots separated by a transactional barrier.
    super::PENDING.with(|pending| assert_eq!(pending.get(), 4));
    let main_loop = glib::MainLoop::new(None, false);
    super::flush_pending(Duration::from_millis(60), {
        let main_loop = main_loop.clone();
        move |result| {
            assert_eq!(result, Err(super::FlushError::TimedOut));
            main_loop.quit();
        }
    });
    main_loop.run();
    assert!(writes.lock().unwrap().is_empty());
    release.send(()).unwrap();
    super::flush_pending(Duration::from_secs(2), {
        let main_loop = main_loop.clone();
        move |result| {
            assert_eq!(result, Ok(()));
            main_loop.quit();
        }
    });
    main_loop.run();
    assert_eq!(*writes.lock().unwrap(), [999, 1000, 1999]);
    super::PENDING.with(|pending| assert_eq!(pending.get(), 0));
}

#[test]
fn failure_acknowledgement_preserves_evidence_and_does_not_hide_new_failures() {
    if !super::isolated_test_child("ui::background_io::tests::failure_acknowledgement_preserves_evidence_and_does_not_hide_new_failures") { return; }
    fn drain(expected: Result<(), super::FlushError>) {
        let main_loop = glib::MainLoop::new(None, false);
        super::flush_pending(Duration::from_secs(2), {
            let main_loop = main_loop.clone();
            move |outcome| {
                assert_eq!(outcome, expected);
                main_loop.quit();
            }
        });
        main_loop.run();
    }
    super::run_serialized(
        super::SerialLane::Config,
        || panic!("first failed save"),
        |(): ()| {},
    );
    drain(Err(super::FlushError::WorkerFailed));
    let decision = super::failure_evidence();
    super::run_serialized(super::SerialLane::Config, || (), |()| {});
    drain(Err(super::FlushError::WorkerFailed));
    super::run_serialized(
        super::SerialLane::Config,
        || panic!("later failed save"),
        |(): ()| {},
    );
    drain(Err(super::FlushError::WorkerFailed));
    super::acknowledge_failures(decision);
    drain(Err(super::FlushError::WorkerFailed));
    super::acknowledge_failures(super::failure_evidence());
    drain(Ok(()));
    super::WORKER_FAILURES
        .with(|failures| assert_eq!(failures.get(), 2, "acknowledgement must preserve evidence"));
}

#[test]
fn flush_waits_for_workers_gtk_publication_and_chained_writes() {
    if !super::isolated_test_child(
        "ui::background_io::tests::flush_waits_for_workers_gtk_publication_and_chained_writes",
    ) {
        return;
    }
    let main_loop = glib::MainLoop::new(None, false);
    let published = std::rc::Rc::new(std::cell::Cell::new(false));
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    super::run(move || release_rx.recv().unwrap(), {
        let published = published.clone();
        move |()| {
            super::run_serialized(
                super::SerialLane::Config,
                || (),
                move |()| published.set(true),
            )
        }
    });
    super::flush_pending(Duration::from_millis(40), {
        let main_loop = main_loop.clone();
        move |result| {
            assert_eq!(result, Err(super::FlushError::TimedOut));
            main_loop.quit();
        }
    });
    main_loop.run();
    assert!(!published.get());
    release_tx.send(()).unwrap();
    super::flush_pending(Duration::from_secs(2), {
        let main_loop = main_loop.clone();
        let published = published.clone();
        move |result| {
            assert_eq!(result, Ok(()));
            assert!(published.get());
            main_loop.quit();
        }
    });
    main_loop.run();
}

#[test]
fn flush_reports_panicked_work_and_registry_lane_survives() {
    if !super::isolated_test_child(
        "ui::background_io::tests::flush_reports_panicked_work_and_registry_lane_survives",
    ) {
        return;
    }
    super::run_serialized(
        super::SerialLane::Registry,
        || panic!("injected failure"),
        |()| panic!("must not publish"),
    );
    let completed = std::rc::Rc::new(std::cell::Cell::new(false));
    super::run_serialized(super::SerialLane::Registry, || (), {
        let completed = completed.clone();
        move |()| completed.set(true)
    });
    let main_loop = glib::MainLoop::new(None, false);
    super::flush_pending(Duration::from_secs(2), {
        let main_loop = main_loop.clone();
        move |result| {
            assert_eq!(result, Err(super::FlushError::WorkerFailed));
            assert!(completed.get());
            main_loop.quit();
        }
    });
    main_loop.run();
}

#[test]
fn work_runs_off_the_calling_thread() {
    let caller = std::thread::current().id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        tx.send(std::thread::current().id()).expect("worker id");
    });
    assert_ne!(rx.recv().expect("worker id"), caller);
}

/// A worker that panics must be observable, or the GTK timer polls forever.
///
/// `run` cannot be called here — it needs a GTK main loop — so this pins
/// the channel behaviour `run` relies on instead: a worker that unwinds
/// drops its sender, and the receiver reports `Disconnected` rather than
/// staying `Empty`.
#[test]
fn a_panicking_worker_disconnects_rather_than_staying_silent() {
    let (sender, receiver) = std::sync::mpsc::sync_channel::<()>(1);
    let worker = std::thread::spawn(move || {
        let _sender = sender;
        panic!("worker failed");
    });
    assert!(worker.join().is_err(), "the worker should have panicked");

    assert!(matches!(
        receiver.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Disconnected)
    ));
}

/// A value sent before a panic still arrives.
///
/// This is what keeps the happy path identical to the shared-slot version
/// it replaced: `try_recv` yields buffered data before it reports that the
/// sender is gone, so a worker that panics *after* sending still delivers.
#[test]
fn a_value_sent_before_a_panic_is_still_delivered() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        sender.send("done").expect("send");
        panic!("worker failed after sending");
    });
    assert!(worker.join().is_err(), "the worker should have panicked");

    assert_eq!(receiver.try_recv(), Ok("done"));
}

#[test]
fn serialized_jobs_do_not_overtake_a_blocked_predecessor() {
    let queue = super::SerialQueue::new("scenedeck-test-serialized-io");
    let (first_started_tx, first_started_rx) = std::sync::mpsc::channel();
    let (release_first_tx, release_first_rx) = std::sync::mpsc::channel();
    let (finished_tx, finished_rx) = std::sync::mpsc::channel();

    queue.enqueue(Box::new({
        let finished_tx = finished_tx.clone();
        move || {
            first_started_tx.send(()).expect("report first start");
            release_first_rx.recv().expect("release first job");
            finished_tx.send("first").expect("report first finish");
        }
    }));
    first_started_rx.recv().expect("first job started");

    queue.enqueue(Box::new(move || {
        finished_tx.send("second").expect("report second finish");
    }));

    assert_eq!(
        finished_rx.recv_timeout(Duration::from_millis(50)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout),
        "the second job ran while its predecessor was still blocked"
    );

    release_first_tx.send(()).expect("release first job");
    assert_eq!(finished_rx.recv().expect("first result"), "first");
    assert_eq!(finished_rx.recv().expect("second result"), "second");
}

#[test]
fn latest_serialized_file_snapshot_wins() {
    let queue = super::SerialQueue::new("scenedeck-test-latest-snapshot");
    let dir = std::env::temp_dir().join(format!(
        "scenedeck-background-io-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("config.json");
    let (first_started_tx, first_started_rx) = std::sync::mpsc::channel();
    let (release_first_tx, release_first_rx) = std::sync::mpsc::channel();
    let (finished_tx, finished_rx) = std::sync::mpsc::channel();

    queue.enqueue(Box::new({
        let path = path.clone();
        let finished_tx = finished_tx.clone();
        move || {
            first_started_tx.send(()).expect("report first start");
            release_first_rx.recv().expect("release first snapshot");
            crate::storage::atomic::write(&path, b"older").expect("write older snapshot");
            finished_tx.send(()).expect("report first finish");
        }
    }));
    first_started_rx.recv().expect("first snapshot started");

    queue.enqueue(Box::new({
        let path = path.clone();
        move || {
            crate::storage::atomic::write(&path, b"newer").expect("write newer snapshot");
            finished_tx.send(()).expect("report second finish");
        }
    }));

    release_first_tx.send(()).expect("release first snapshot");
    finished_rx.recv().expect("first snapshot finished");
    finished_rx.recv().expect("second snapshot finished");

    assert_eq!(std::fs::read(&path).expect("read final snapshot"), b"newer");
    std::fs::remove_dir_all(dir).expect("remove temp dir");
}
