# Temporary advisory exception

Only [RUSTSEC-2026-0374](https://rustsec.org/advisories/RUSTSEC-2026-0374.html)
is accepted, owned by the **SceneDeck maintainer**, until **2026-11-20
00:00:00 UTC** (exclusive). This is a temporary acceptance of an unmaintained
build-time parser, not a vulnerability fix or a claim that the parser is safe.

The 2026-10-20 review found no compatible published localization release that
removes `basic-toml`. The replacement in
[cargo-i18n PR #176](https://github.com/kellpossible/cargo-i18n/pull/176) was
unreleased. The reviewed crates.io packages are:

- `basic-toml 0.1.10`, used only by `i18n-config 0.4.8`.
- `i18n-config 0.4.8`, used only by proc macros `i18n-embed-fl 0.10.1` and
  `i18n-embed-impl 0.8.4`.
- `i18n-embed-impl 0.8.4`, used only by `i18n-embed 0.16.0`.
- `i18n-embed 0.16.0`, used by SceneDeck and `i18n-embed-fl 0.10.1`;
  `i18n-embed-fl 0.10.1` is otherwise used only by SceneDeck.

The pinned registry sources and archive checksums preserve the reviewed
proc-macro manifests. Any change to these package identities or incoming paths
requires removing or re-reviewing the exception. Unrelated packages are not
frozen.

## Enforcement

`tools/advisory-exception.toml` records the owner, reason and deadline.
`tools/check-advisory-exceptions.py` requires an exact matching single ignore in
`deny.toml`; rejects missing approval metadata, added ignores and graph drift;
and fails on or after the deadline using the actual UTC system clock. There is
no CLI or environment clock override, and changing the record cannot extend the
hard-coded approved deadline.

CI runs the checker before the dependency scanners. Release and Snap workflows
inherit it through reusable CI. For the same bounded local policy, run:

```sh
python3 tools/check-advisory-exceptions.py &&
  cargo audit &&
  cargo deny --locked --all-features check advisories bans licenses sources
```

`cargo-deny 0.20.2` does not enforce dates in ignore objects. Running it directly
without the checker does **not** enforce this expiry. Yanked crates and unused
ignores remain denied; source, license and bans policies are unchanged.
`cargo audit` receives no ignore flags, so this exception does not suppress a
future vulnerability classification there.

Remove the exception earlier when a compatible fixed release is published, or
immediately re-review if the advisory is reclassified. Verify the published
archive and update dependencies under normal validation, then remove the
ignore, approval record and dedicated checker/fixtures/CI contract together in
a reviewed change. Do not silently extend the deadline.
