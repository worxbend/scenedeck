//! Command failure messages shown to the operator.

use crate::ui::window::*;

#[test]
fn command_failure_message_shows_the_error_once() {
    let error = AppError::Request("boom".into());
    let detail = error.user_message();
    for operation in [
        FailedOperation::Command,
        FailedOperation::Profile,
        FailedOperation::ProfileReadback,
        FailedOperation::Collection,
        FailedOperation::CollectionReadback,
    ] {
        let message = command_failure_message(operation, &error);
        assert_eq!(
            message.matches(&detail).count(),
            1,
            "{operation:?}: {message}"
        );
    }
    assert!(command_failure_message(FailedOperation::Profile, &error)
        .starts_with(&fl!(LANGUAGE_LOADER, "window-selector-profile-label")));
    assert!(
        command_failure_message(FailedOperation::CollectionReadback, &error)
            .starts_with(&fl!(LANGUAGE_LOADER, "window-selector-collection-label"))
    );
}
