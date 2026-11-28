# Deep review progress

Progress against [deep-review-2026-11-14.md](deep-review-2026-11-14.md), on
branch `refactor/deep-review-fixes`. Status is one of done, already_done,
blocked, failed or skipped. `already_done` means the work landed on `main`
before the branch was cut (commits listed where known).

## P0 - Urgent fix and safety nets

| Task | Status | Commits | Notes |
|---|---|---|---|
| P0-T01 | already_done | a29dea4 (main) | RUSTSEC-2026-0374 exception renewed to 2026-12-20. Re-checked 2026-11-27: still no i18n-config release. Re-check before 2026-12-20. cargo-deny newer than 0.20.2 reports this ignore as advisory-not-detected. |
| P0-T02 | done | 5559f8f | `native_test!` takes only the closure; the path is derived from the enclosing fn. 8 native tests still spawn their own child with a hard-coded path (theme, studio_controls, transport_bar, preview_scheduler, settings); convert them before splitting those files. |
| P0-T03 | already_done | a58adcf, 9369b9e (main) | Inventory self-scan removed; `inventory_does_not_use_a_column_view` scanner rule. `storage/registry.rs` still has an include_str self-scan. |
| P0-T04 | already_done | 9369b9e (main) | Scanner split with `Forbidden`, exempt prefixes and `ratchet`. Move and behavior change share one commit on main (not rewritten). |
| P0-T05 | done | b7b5e59 (main), bafd325 | Golden fixtures plus `enum-spellings.json`. Six enums are listed by hand and do not force a fixture entry for a new variant. |
| P0-T06 | done | 05d3314, 9128666 | Fixture moved to `obs::testing::wire`; `ScriptedObs` with typed replies and request counts. `control_tests::mock_client` shim remains for P2-T04. |
| P0-T07 | done | 8d59180, 85ba889, 6d52c7c | Four dependency rules and four ratchets (file budget, AppState borrows, thread_local, domain presentation). Text ratchets count test code and comments; no test enforces that a ceiling only goes down. |
| P0-T08 | already_done | c99d8d0 (main) | `NavigationContext::read`/`update`; no callers migrated yet. |
| P0-T09 | done | f6dc127 (main), 804a87b | `lock_or_recover` landed on main; docs added here. `.lock().expect` remains in app_controller queues, workspace `queued` and obs/requests.rs (P1-T05 / later). |
| P0-T10 | already_done | b126c40 (main) | lifecycle.rs documents its per-mutex poison policy and lock order. |
| P0-T11 | done | 49c5585 | 22 event-loop characterization tests over `ScriptedObs::connect_with_events`. Lag/resync branch and coalescing not covered. |
| P0-T12 | done | a009601 | Golden AppEvent trace through the AppState reducers, including stop-ticket status. Pins two current quirks (duplicate observed recording result; readback failure leaves the stream clock running). |
| P0-T13 | done | 441ed48, ec576ee | Native characterization tests for Sources and Inventory sections. YAML dialog completion not covered. |
| P0-T14 | done | 1fecc3b, c258bde, 66202b9, bce8284, 519ee92 | Shared ui, controller and storage test support; storage->controller test edge removed. |
| P0-T15 | done | f447a6b, 8995729, ce1659f, 98afa37 | Opt-in `perf-trace` feature; releases build default features; CI lints the default feature set. live.rs and mixer.rs spans deferred to their splits (files at ratchet ceiling). |
| Gate | done | fcaaa34 | Phase gate: SV, full NV, IV, cargo audit and cargo deny green. Doc fix: storage golden fixtures listed in codebase-overview. Rule probe: added debt failed the new rules, then reverted. |

## P1 - Correctness, security and data-safety fixes

| Task | Status | Commits | Notes |
|---|---|---|---|
| P1-T01 | done | 578be5c | `ui::markup::plain_toast` and `widgets::status_page`; rule `toasts_and_status_pages_go_through_the_literal_helpers`. transport_bar.rs still calls `glib::markup_escape_text` directly; ActionRow/ExpanderRow markup has no rule. |
| P1-T02 | done | a2b5770 | `OutputReadbackErrors` is `#[must_use]`; manual RefreshData publishes it. Shared scan left for P7-T07. |
| P1-T03 | done | ebb32d7, ee59986, 6f2cbf8 | Decision 1: scene cards and hotkeys stay gated on Ready. Became tests, docs and a no-op refactor. Gate rule still lives in several places (P8/P9). |
| P1-T04 | done | fb532da | `ui::persist::retry_all_durable_writes` re-admits registry, history, exports, config and bookmarks. Anonymous worker panics still need "Close anyway" (P9-T14/T16). |
| P1-T05 | done | 4024134 | `controller::runtime::SingleFlight` with a `Worker` guard; refused spawns publish their failure. The request in flight when a session ends still gets no answer. |
| P1-T06 | done | 4aff377 | Controller config reads return `Result`. `ObsSessionRunner` still connects with defaults (logged) when config is unusable. |
| P1-T07 | done | 70b04f4, 4f4e902 | Pure move into `ui::presenters::output_status`, then one `render_output_slot`. Clock move to `ui/shell/ticks` deferred to the window.rs split. |
| P1-T08 | done | 97a5294, 7def2c1 | Newer config.json kept intact; registry.json gains a version envelope (format change). |
| P1-T09 | done | 24c070f | Config reads are pure; migration write and move-aside run only at startup. A second non-primary instance can still move a broken config aside. |
| P1-T10 | done | f2de086 | Decision 3: `BTreeMap` in the registry snapshot and scene graph; byte-wise scene-id order. |
| P1-T11 | done | a7384f1 | Control snapshot uses `mapper::map_scenes`. Other inline obws conversions left for obs-acl-08. |
| P1-T12 | done | 3bbf089, 094f34a | Named capacities; `AdmissionRefused::{Inactive, JournalFull}`; a full journal reports `OutputCommandFailed`. Audio commands on a full journal are still only logged. |
| P1-T13 | done | 6dac451, fbe5713, 2a5bc41 | `ObsPassword` redacts Debug; `AppState::obs_password` and the startup keyring read removed. The Settings editor loads the password lazily on the Secret lane. A wedged keyring keeps the connection group insensitive until the read answers. |
| P1-T14 | done | 5f31c81, 88c4f78 | One `CredentialStore` port (`KeyringCredentialStore`); rule `the_keyring_is_reached_only_through_the_credential_store`. Settings Connect waits on the FIFO Secret lane. The sidebar Connect button and Ctrl+R reconnect still skip that barrier. Controller-only fakes implement write/delete as panics (P4-T22). |
| P1-T15 | done | d8d4125, d067424 | No keyring read before the first frame (landed with P1-T13); typed `StartupReadError` with a translated reader-panic text; native locked-keyring startup test. Step 4 (bookmarks/history after first frame) left to P9-T08. `Store(String)` still carries untranslated store text. |
| P1-T16 | done | 32a9ee4 | `domain::connection::endpoint::is_loopback`; Settings warns when the saved host is not loopback. Recommends SSH tunnel or VPN (no wss://, decision 10). Takes `&str` until ObsHost exists. |
| P1-T17 | done | e2dc714 | XDG paths: relative values ignored, no working-directory fallback; last resort is `/` with a warning. |
| P1-T18 | done | 452d0c9 | Relative custom CSS paths refused with a localized warning. A stored relative path warns on every apply. |
| P1-T19 | done | fd9658a | `storage::registry::import` with a parser budget and caps. Export is not capped, so an over-cap registry exports but will not re-import. |
| P1-T20 | done | 401ac80 | Show-workspace store capped at 64 with LRU eviction and a toast (format change). `now_unix_ms` duplicated in storage and controller. |
| P1-T21 | done | 3c9754d | `storage::lock` sibling lock files; Busy shown where save errors already show. Failed `persist_config` saves still have no toast. |
| P1-T22 | done | c44b93a | Decision 9: symlinks still refused; `ConfigStartupNotice::Symlinked`. Other stores still report a generic ELOOP. |
| P1-T23 | done | 31eae55 | Unreadable registry.json becomes a startup notice; `startup_notices: Vec<String>`; new `settings/startup_notices.rs`. |
| P1-T24 | done | 3c24d97 | Scene search saves once after 1 s settle; flushed on close. A query typed under 1 s before a workspace switch stays in memory only. |
| P1-T25 | done | 6a20eb0 | Decision 8: endpoint and host paths redacted in exports by default, two opt-in check buttons. Free-text redaction only covers values the report holds. |
| Gate | done | 20bd0c2 | SV, full NV (123 native tests), IV, cargo audit (0.22.2, warnings only) and cargo deny (0.20.2, the CI pin) green. cargo-deny 0.22 reports the RUSTSEC-2026-0374 ignore as not detected (known since P0). Fix: ui/persist.rs had grown from 578 to 723 production lines; pure move of config saves and workspace selection into `persist/config.rs` and `persist/registry_scope.rs` (now 576; test count 1036/125 before and after). Ratchets only went down. Still over 600 and grown this phase, left for their planned splits: storage/registry.rs (646 to 691, P5) and ui/theme.rs (616 to 624). Manual OBS smoke run not done (needs a human). |
| Gate (rerun) | done | - | After P1-T13..T16 landed: SV (914 unit tests pass, 128 ignored), full NV (126 native tests), IV, cargo audit 0.22.2 (6 allowed warnings) and cargo deny 0.20.2 green. No fixes needed. Ratchets only went down (state.rs 1221 to 1214, settings.rs 822 to 821, connections.rs borrows [5,16,22] to [0,13,14]); window.rs grew 1519 to 1520, at its unchanged ceiling. No production file passed 600 lines in these tasks (app.rs 502 to 542, connections.rs 447 to 481). Docs match the tree. Stashes stash@{0..2} (crashed-before-P1-T13/T17/T07) are left for their owners. Manual OBS smoke run not done (needs a human). |
