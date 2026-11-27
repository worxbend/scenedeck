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
