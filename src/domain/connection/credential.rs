//! The OBS WebSocket password as a value that does not print itself.

use std::fmt;

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
