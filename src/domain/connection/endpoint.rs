//! Facts about an OBS WebSocket endpoint that need no network access.

use std::net::{IpAddr, Ipv6Addr};

/// Whether `host` names this machine, so OBS traffic to it never leaves the
/// loopback interface.
///
/// obs-websocket serves plain `ws://`: the authentication exchange and every
/// command travel unencrypted. Only a loopback host keeps them off the
/// network, so any host this cannot prove local is treated as remote. That
/// includes names such as `obs.local` or `*.localhost` whose resolution
/// depends on the system resolver.
///
/// Accepted: `localhost` in any case with an optional trailing dot, any IPv4
/// address in `127.0.0.0/8`, `::1` with or without brackets, and IPv4-mapped
/// loopback such as `::ffff:127.0.0.1`. The host is not trimmed, because the
/// connection uses it exactly as written.
pub(crate) fn is_loopback(host: &str) -> bool {
    if host
        .strip_suffix('.')
        .unwrap_or(host)
        .eq_ignore_ascii_case("localhost")
    {
        return true;
    }
    let address = match host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        Some(bracketed) => bracketed.parse::<Ipv6Addr>().map(IpAddr::V6),
        None => host.parse::<IpAddr>(),
    };
    address.is_ok_and(|address| address.to_canonical().is_loopback())
}

#[cfg(test)]
mod tests {
    use super::is_loopback;

    #[test]
    fn loopback_names_and_addresses_are_local() {
        for host in [
            "127.0.0.1",
            "127.4.5.6",
            "::1",
            "[::1]",
            "::ffff:127.0.0.1",
            "localhost",
            "LocalHost",
            "localhost.",
        ] {
            assert!(is_loopback(host), "{host} is loopback");
        }
    }

    #[test]
    fn lan_public_and_unresolved_hosts_are_remote() {
        for host in [
            "192.168.1.20",
            "10.0.0.5",
            "0.0.0.0",
            "::",
            "[192.168.1.20]",
            "2001:db8::1",
            "obs.local",
            "studio.localhost",
            "localhost.example.com",
            "127.0.0.1.nip.io",
            " 127.0.0.1",
            "",
        ] {
            assert!(!is_loopback(host), "{host:?} is remote");
        }
    }
}
