//! Text presenters shared by more than one GTK surface.
//!
//! A presenter turns domain values into the exact string a widget shows. It
//! builds no widgets and reads no `AppState`, so every surface that shows the
//! same value renders the same text, and tests need no display.

pub(crate) mod output_status;
pub(crate) mod save_error;
