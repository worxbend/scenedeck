use super::navigation::NavigationContext;
use crate::controller::{app_controller::AppController, event::AppEvent, state::AppState};
use std::{cell::RefCell, rc::Rc, sync::mpsc, time::Duration};

/// Native tests need the isolated display and session bus that
/// `tools/test-native.py` sets up; on a live desktop they fail confusingly.
pub(crate) fn require_native_runner() {
    assert!(
        std::env::var_os("SCENEDECK_NATIVE_RUNNER").is_some(),
        "native GTK tests must run via `python3 tools/test-native.py`"
    );
}

/// GTK tests run in separate processes, not separate Rust test threads.
///
/// `module_and_fn` is the test fn's full path from `native_test!`, so it
/// starts with the crate name. Libtest's `--exact` filter omits that name.
/// A stale filter used to exit 0 with `0 passed`; that is a failure.
pub(crate) fn native_test(module_and_fn: &str, test: impl FnOnce()) {
    require_native_runner();
    let name = libtest_filter(module_and_fn);
    if std::env::var("SCENEDECK_NATIVE_TEST_CHILD").as_deref() != Ok(name) {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--ignored", "--test-threads=1"])
            .env("SCENEDECK_NATIVE_TEST_CHILD", name)
            .env("GTK_A11Y", "test")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!(
                    "native child timed out: {}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_one_passed(name, &child.wait_with_output().unwrap());
        return;
    }
    adw::init().expect("native GTK display");
    gtk4::prelude::ObjectExt::set_property(
        &gtk4::Settings::default().unwrap(),
        "gtk-enable-animations",
        false,
    );
    super::register_resources();
    test();
}

pub(crate) fn navigation() -> (
    NavigationContext,
    gtk4::Stack,
    tokio::runtime::Runtime,
    mpsc::Receiver<AppEvent>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (tx, rx) = mpsc::sync_channel(128);
    let controller = Rc::new(RefCell::new(AppController::new(
        runtime.handle().clone(),
        tx,
    )));
    let state = Rc::new(RefCell::new(AppState::new(
        Default::default(),
        Default::default(),
        None,
        None,
    )));
    let stack = gtk4::Stack::new();
    (
        NavigationContext::new(state, stack.clone(), controller),
        stack,
        runtime,
        rx,
    )
}

pub(crate) fn pump(duration: Duration) {
    let until = std::time::Instant::now() + duration;
    let context = glib::MainContext::default();
    while std::time::Instant::now() < until {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Pump until `ready` holds or `timeout` passes; true when ready.
pub(crate) fn pump_until(timeout: Duration, mut ready: impl FnMut() -> bool) -> bool {
    let until = std::time::Instant::now() + timeout;
    while !ready() {
        if std::time::Instant::now() >= until {
            return false;
        }
        pump(Duration::from_millis(10));
    }
    true
}

/// `scenedeck::ui::tests::native_example::anchor` names an item declared in
/// `native_example`; drop the item to get the test fn's path.
pub(crate) fn enclosing_fn_path(item_path: &str) -> &str {
    let Some((fn_path, _)) = item_path.rsplit_once("::") else {
        panic!("native test anchor must be declared inside a test fn, got {item_path}");
    };
    fn_path
}

/// Drop the crate segment from `scenedeck::ui::tests::native_example`.
pub(crate) fn libtest_filter(module_and_fn: &str) -> &str {
    let Some((_, rest)) = module_and_fn.split_once("::") else {
        panic!("native test path must be crate::module::function, got {module_and_fn}");
    };
    if rest.is_empty() || !rest.contains("::") {
        panic!("native test path must include a module and function, got {module_and_fn}");
    }
    rest
}

pub(crate) fn passed_count(output: &str) -> Option<u32> {
    output.lines().rev().find_map(|line| {
        let (_, summary) = line.split_once("test result:")?;
        let (before, _) = summary.split_once("passed")?;
        before.split_whitespace().last()?.parse().ok()
    })
}

fn assert_one_passed(name: &str, output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let passed = passed_count(&stdout).or_else(|| passed_count(&stderr));
    assert!(
        output.status.success() && passed == Some(1),
        "native child {name} reported {passed:?} passed, expected 1\n{stdout}\n{stderr}"
    );
}

#[test]
fn native_test_path_names_the_enclosing_test_fn() {
    assert_eq!(
        crate::ui::native_test!(@path),
        concat!(
            module_path!(),
            "::native_test_path_names_the_enclosing_test_fn"
        )
    );
}

#[test]
#[should_panic(expected = "must be declared inside a test fn")]
fn enclosing_fn_path_rejects_a_bare_name() {
    enclosing_fn_path("native_test_anchor");
}

#[test]
fn libtest_filter_strips_the_crate_segment() {
    assert_eq!(
        libtest_filter("scenedeck::ui::tests::native_example"),
        "ui::tests::native_example"
    );
}

#[test]
#[should_panic(expected = "must be crate::module::function")]
fn libtest_filter_rejects_a_bare_name() {
    libtest_filter("native_example");
}

#[test]
#[should_panic(expected = "must include a module and function")]
fn libtest_filter_rejects_a_path_without_a_module() {
    libtest_filter("scenedeck::native_example");
}

#[test]
fn passed_count_reads_the_last_libtest_summary() {
    let summary = |passed: u32| {
        format!(
            "test result: ok. {passed} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s"
        )
    };
    assert_eq!(passed_count(&summary(0)), Some(0));
    assert_eq!(passed_count(&summary(1)), Some(1));
    assert_eq!(passed_count(&summary(11)), Some(11));
    assert_eq!(
        passed_count(&format!("{}\n{}", summary(1), summary(11))),
        Some(11)
    );
    assert_eq!(passed_count("1 passed without a summary"), None);
}

#[test]
fn stale_native_path_fails_when_the_child_reports_zero_passed() {
    let previous = std::env::var_os("SCENEDECK_NATIVE_RUNNER");
    std::env::set_var("SCENEDECK_NATIVE_RUNNER", "1");
    let panicked = std::panic::catch_unwind(|| {
        native_test("scenedeck::not::a::real::native_missing", || {});
    });
    match previous {
        Some(value) => std::env::set_var("SCENEDECK_NATIVE_RUNNER", value),
        None => std::env::remove_var("SCENEDECK_NATIVE_RUNNER"),
    }
    let payload = panicked.expect_err("a path that matches no test must fail");
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("panic payload was not a string");
    assert!(
        message.contains("expected 1") && message.contains("Some(0)"),
        "{message}"
    );
}
