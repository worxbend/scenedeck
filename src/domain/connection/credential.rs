//! The OBS WebSocket password as a value that does not print itself, and the
//! one port through which it is stored.

use std::fmt;

use crate::infra::error::AppError;

/// Where each saved connection's OBS password is kept, keyed by its
/// immutable connection ID.
///
/// The controller reads through it at connect time and the Settings
/// connection editor reads and writes through it, so both see the same
/// store. Calls block on D-Bus: make them only from Tokio's blocking pool or
/// the Secret lane, never from the GTK thread or an async task.
pub(crate) trait CredentialStore: Send + Sync {
    /// The stored password, or `None` when the connection has none.
    fn read(&self, connection_id: &str) -> Result<Option<ObsPassword>, AppError>;

    /// Store `password`, replacing any earlier one.
    fn write(&self, connection_id: &str, password: &ObsPassword) -> Result<(), AppError>;

    /// Remove the stored password. Removing one that does not exist succeeds.
    fn delete(&self, connection_id: &str) -> Result<(), AppError>;
}

/// An OBS WebSocket password.
///
/// `Debug` prints a placeholder and there is no `Display`, so a password held
/// in a logged or panicking structure cannot leak through formatting. Read
/// the text with [`ObsPassword::expose`] only where it must leave as text:
/// the keyring, the OBS handshake and the Settings password field.
///
/// The text is not zeroed on drop. `obws` copies it into its own `String` at
/// connect time, so zeroing this copy alone would not clear the password from
/// memory.
#[derive(Clone)]
pub(crate) struct ObsPassword(Box<str>);

impl ObsPassword {
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl From<String> for ObsPassword {
    fn from(password: String) -> Self {
        Self(password.into_boxed_str())
    }
}

impl From<&str> for ObsPassword {
    fn from(password: &str) -> Self {
        Self(password.into())
    }
}

impl fmt::Debug for ObsPassword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ObsPassword(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::ObsPassword;

    #[test]
    fn debug_prints_a_placeholder_instead_of_the_password() {
        let password = ObsPassword::from("hunter2-fixture");

        assert_eq!(format!("{password:?}"), "ObsPassword(***)");
        assert_eq!(format!("{:?}", Some(password)), "Some(ObsPassword(***))");
    }

    #[test]
    fn expose_returns_the_exact_text() {
        assert_eq!(ObsPassword::from(" spaced ").expose(), " spaced ");
        assert_eq!(ObsPassword::from(String::from("zażółć")).expose(), "zażółć");
    }
}
