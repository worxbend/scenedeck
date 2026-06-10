//! Typed channel aliases for the UI ↔ Controller message bus.
//!
//! Phase 1: type aliases only — nothing is wired yet.
//! Phase 2: the AppController will own the receiver end; the UI will hold
//! a cloned sender and call `nav.dispatch()` which forwards over the channel.

use tokio::sync::mpsc;

use crate::controller::command::AppCommand;
use crate::controller::event::AppEvent;

pub type CommandSender = mpsc::Sender<AppCommand>;
pub type CommandReceiver = mpsc::Receiver<AppCommand>;
pub type EventSender = mpsc::Sender<AppEvent>;
pub type EventReceiver = mpsc::Receiver<AppEvent>;

pub fn command_channel(buf: usize) -> (CommandSender, CommandReceiver) {
    mpsc::channel(buf)
}

pub fn event_channel(buf: usize) -> (EventSender, EventReceiver) {
    mpsc::channel(buf)
}
