//! Asynchronous XDG GlobalShortcuts v1 client using the existing GIO bus stack.
//! Request subscriptions precede calls; every session/request has scoped cleanup.

use gio::prelude::*;
use glib::{variant::ObjectPath, Variant};
use std::{cell::RefCell, collections::HashMap, time::Duration};

type Dict = HashMap<String, Variant>;
pub(super) type Shortcuts = Vec<(String, Dict)>;
const BUS: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.portal.GlobalShortcuts";

pub(super) struct Portal {
    proxy: gio::DBusProxy,
    owner: String,
    _connection: OwnedConnection,
}

/// A dedicated peer allows Register to precede any GTK portal calls. Closing
/// the peer also retires sessions if cancellation races their creation.
struct OwnedConnection(gio::DBusConnection);

impl Drop for OwnedConnection {
    fn drop(&mut self) {
        let connection = self.0.clone();
        self.0.flush(gio::Cancellable::NONE, move |_| {
            connection.close(gio::Cancellable::NONE, |_| {});
        });
    }
}

/// Closing on drop also covers cancellation while a portal dialog is open.
struct RemoteHandle {
    connection: gio::DBusConnection,
    owner: String,
    path: String,
    interface: &'static str,
    close: bool,
}

impl Drop for RemoteHandle {
    fn drop(&mut self) {
        if self.close {
            self.connection.call(
                Some(&self.owner),
                &self.path,
                self.interface,
                "Close",
                None,
                None,
                gio::DBusCallFlags::NONE,
                3000,
                gio::Cancellable::NONE,
                |_| {},
            );
        }
    }
}

pub(super) struct Session {
    handle: RemoteHandle,
    portal: Portal,
}

impl Portal {
    pub(super) async fn connect() -> Result<Self, String> {
        let address = gio::spawn_blocking(|| {
            gio::dbus_address_get_for_bus_sync(gio::BusType::Session, gio::Cancellable::NONE)
        })
        .await
        .map_err(|_| "Session bus address lookup failed")?
        .map_err(|e| e.to_string())?;
        let connection = OwnedConnection(
            gio::DBusConnection::for_address_future(
                &address,
                gio::DBusConnectionFlags::AUTHENTICATION_CLIENT
                    | gio::DBusConnectionFlags::MESSAGE_BUS_CONNECTION,
                None,
            )
            .await
            .map_err(|e| e.to_string())?,
        );
        connection.0.set_exit_on_close(false);
        // Older portals lack Registry; sandboxed apps derive identity from their
        // sandbox and reject Register. In either case CreateSession remains the
        // authority on admission; failure here never bypasses portal permission.
        if let Err(error) = connection
            .0
            .call_future(
                Some(BUS),
                PATH,
                "org.freedesktop.host.portal.Registry",
                "Register",
                Some(&(crate::app_info::APP_ID, Dict::new()).to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                10000,
            )
            .await
        {
            tracing::debug!(%error, "Portal host registration unavailable; using portal-derived identity");
        }
        let proxy = gio::DBusProxy::new_future(
            &connection.0,
            gio::DBusProxyFlags::NONE,
            None,
            Some(BUS),
            PATH,
            INTERFACE,
        )
        .await
        .map_err(|e| e.to_string())?;
        let version = proxy
            .cached_property("version")
            .and_then(|v| v.get::<u32>())
            .unwrap_or(0);
        if version == 0 {
            return Err("GlobalShortcuts portal is unavailable".into());
        }
        let owner = proxy.name_owner().ok_or("Portal has no owner")?.to_string();
        Ok(Self {
            proxy,
            owner,
            _connection: connection,
        })
    }

    fn handle(&self, path: String, interface: &'static str) -> RemoteHandle {
        RemoteHandle {
            connection: self.proxy.connection(),
            owner: self.owner.clone(),
            path,
            interface,
            close: true,
        }
    }

    fn token_path(&self, kind: &str, token: &str) -> Result<String, String> {
        let sender = self
            .proxy
            .connection()
            .unique_name()
            .ok_or("Bus has no unique name")?
            .trim_start_matches(':')
            .replace('.', "_");
        Ok(format!("{PATH}/{kind}/{sender}/{token}"))
    }

    async fn request(&self, method: &str, token: &str, args: Variant) -> Result<Dict, String> {
        let mut request = self.handle(
            self.token_path("request", token)?,
            "org.freedesktop.portal.Request",
        );
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = RefCell::new(Some(tx));
        let _subscription = request.connection.subscribe_to_signal(
            Some(&self.owner),
            Some(request.interface),
            Some("Response"),
            Some(&request.path),
            None,
            gio::DBusSignalFlags::NONE,
            move |signal| {
                if let Some(tx) = tx.borrow_mut().take() {
                    let _ = tx.send(signal.parameters.clone());
                }
            },
        );
        let reply = self
            .proxy
            .connection()
            .call_future(
                Some(&self.owner),
                PATH,
                INTERFACE,
                method,
                Some(&args),
                None,
                gio::DBusCallFlags::NONE,
                10000,
            )
            .await
            .map_err(|e| e.to_string())?;
        let (returned,) = reply
            .get::<(ObjectPath,)>()
            .ok_or("Invalid request handle")?;
        // GlobalShortcuts postdates the token-based request-path convention.
        if returned.as_str() != request.path {
            return Err("Unexpected portal request path".into());
        }
        let reply = glib::future_with_timeout(Duration::from_secs(120), rx)
            .await
            .map_err(|_| "Portal request timed out")?
            .map_err(|_| "Portal request closed")?;
        let (code, result) = reply
            .get::<(u32, Dict)>()
            .ok_or("Invalid portal response")?;
        request.close = false;
        match code {
            0 => Ok(result),
            1 => Err("Portal request cancelled".into()),
            _ => Err("Portal request refused".into()),
        }
    }

    pub(super) async fn create(self) -> Result<Session, String> {
        let token = token();
        let session_token = token.clone() + "_session";
        let handle = self.handle(
            self.token_path("session", &session_token)?,
            "org.freedesktop.portal.Session",
        );
        let options = HashMap::from([
            ("handle_token", token.to_variant()),
            ("session_handle_token", session_token.to_variant()),
        ]);
        let result = self
            .request("CreateSession", &token, (options,).to_variant())
            .await?;
        if result.get("session_handle").and_then(|v| v.str()) != Some(&handle.path) {
            return Err("Unexpected portal session path".into());
        }
        Ok(Session {
            portal: self,
            handle,
        })
    }
}

impl Session {
    pub(super) fn alive(&self) -> bool {
        self.portal.proxy.name_owner().as_deref() == Some(&self.portal.owner)
            && !self.portal.proxy.connection().is_closed()
    }

    pub(super) async fn bind(&self, shortcuts: Shortcuts) -> Result<Shortcuts, String> {
        let token = token();
        let options = HashMap::from([("handle_token", token.to_variant())]);
        let path = ObjectPath::try_from(self.handle.path.as_str()).map_err(|e| e.to_string())?;
        // Empty parent identifiers are permitted by the portal API. This avoids
        // making Wayland/X11 native handle support a compatibility prerequisite.
        let result = self
            .portal
            .request(
                "BindShortcuts",
                &token,
                (path, shortcuts, "", options).to_variant(),
            )
            .await?;
        result
            .get("shortcuts")
            .and_then(|v| v.get())
            .ok_or_else(|| "Invalid bound shortcuts".into())
    }

    pub(super) fn listen(
        &self,
        callback: impl Fn(Event) + 'static,
    ) -> Vec<gio::SignalSubscription> {
        let callback = std::rc::Rc::new(callback);
        let path = self.handle.path.clone();
        let connection = self.portal.proxy.connection();
        let events = connection.subscribe_to_signal(
            Some(&self.portal.owner),
            Some(INTERFACE),
            None,
            Some(PATH),
            None,
            gio::DBusSignalFlags::NONE,
            {
                let callback = callback.clone();
                move |signal| match signal.signal_name {
                    "Activated" | "Deactivated" => {
                        if let Some((session, id, _, _)) =
                            signal.parameters.get::<(ObjectPath, String, u64, Dict)>()
                        {
                            if session.as_str() == path {
                                callback(Event::Key {
                                    id,
                                    pressed: signal.signal_name == "Activated",
                                });
                            }
                        }
                    }
                    "ShortcutsChanged" => {
                        if let Some((session, shortcuts)) =
                            signal.parameters.get::<(ObjectPath, Shortcuts)>()
                        {
                            if session.as_str() == path {
                                callback(Event::Changed(shortcuts));
                            }
                        }
                    }
                    _ => {}
                }
            },
        );
        let closed = connection.subscribe_to_signal(
            Some(&self.portal.owner),
            Some("org.freedesktop.portal.Session"),
            Some("Closed"),
            Some(&self.handle.path),
            None,
            gio::DBusSignalFlags::NONE,
            move |_| callback(Event::Closed),
        );
        vec![events, closed]
    }
}

pub(super) enum Event {
    Key { id: String, pressed: bool },
    Changed(Shortcuts),
    Closed,
}

fn token() -> String {
    format!("scenedeck_{}", glib::uuid_string_random().replace('-', ""))
}

#[cfg(test)]
mod tests;
