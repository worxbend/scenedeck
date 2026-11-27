# Maintainer decisions for the deep review plan

Answers to the open questions in section 8.2 of
[deep-review-2026-11-14.md](deep-review-2026-11-14.md). Implementers read this
file before every task. A task marked "MAINTAINER DECISION" follows the answer
here.

Decided by the maintainer on 2026-11-26:

| # | Question | Decision |
|---|---|---|
| 1 | P1-T03: scene switching without curation | Keep the current gating. Scene cards and hotkeys stay disabled until the registry is Ready. P1-T03 shrinks to tests and docs that pin this rule. |
| 8 | P1-T25: report export redaction default | Redact the OBS endpoint by default. The user opts in to include it when exporting. |
| 10 | P11-T03: TLS | Skip TLS. Do not add rustls. Record the deferral in P11-T03 and close it. |
| 11 | P8-T01: AppState location | Move AppState to `src/state/` and allow the `state -> controller::{event, command}` edge. No `src/protocol/` module. |

Plan defaults, applied because the maintainer did not ask for a change. Each
default keeps the current documented behavior:

| # | Question | Default |
|---|---|---|
| 2 | P8-T09: retry on an unchanged edit | Keep the documented retry of retained intent. |
| 3 | P1-T10: Doctor order | Make the order deterministic, but keep today's visible order where it is already stable. Note any reordering in the commit body. |
| 4 | P4-T21: report end reason | Stable codes with an `Other(String)` fallback, so old reports still load. |
| 5 | P4-T19: name policies | Keep two explicit policies: bookmarks trimmed with max 120, presets exact with max 80. |
| 6 | P4-T16: meter evidence | Keep both rules as they are. Name each rule in the domain, and do not change which fields either one reads. |
| 7 | P5-T05: curation naming | `domain/curation/`, moving the snapshot types into it. |
| 9 | P1-T22: symlinked config files | Keep refusing them (O_NOFOLLOW policy). |
| 12 | P10-T21: theme ids | Rename display names only. Ids stay the same. |
| 13 | Audio volume source | Keep both `inputVolumeMul` and `inputVolumeDb` fields. |
