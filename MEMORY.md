[pattern] Keep async UI loading state owned by controller events or a reducer; UI pre-marking plus controller events makes stale responses harder to reason about.
[pattern] For operational controls, extract pure decision helpers around safety behavior so defaults and edge cases can be unit-tested outside GTK callbacks.
[learning] Debounced sliders still need immediate local feedback for all volume controls; otherwise non-slider controls appear inert while waiting for OBS reconciliation.
[anti-pattern] Reusing one duplicate-dispatch guard for automatic rebuilds and explicit user actions can accidentally block retries after recoverable failures.
[anti-pattern] Public mirror fields around reducer-owned state drift unless every event path updates the reducer snapshot and the mirror together.
[pattern] Make reducer-owned mirror fields private once accessors exist; compiler-enforced boundaries catch future UI bypasses better than comments.
[learning] UI reconciliation predicates must cover every render source for a page mode; fixing reducer-backed modes can still leave direct-state modes visually stale.
[pattern] A single visible render-source helper keeps Mixer rendering decisions explicit across modes, but event predicates must consume the same helper to avoid duplicated fallback logic.
[learning] When deleting mirror state, replacement tests should assert both the public visible contract and any intentional hidden reducer state that must survive masked loading/error views.
[learning] Scene-specific refresh-target helpers should be named distinctly from display-source helpers; Active mode can visibly follow a scene while still having no scene-refresh target.
[learning] Blocked manual runs are useful evidence only when they preserve non-claims; keep runtime risks open until a verified environment executes the interaction cases.
[pattern] Gate UI performance optimizations behind observed runtime churn when the current path is correct; otherwise extra widget bookkeeping can add complexity without proof of value.
[learning] State-level reason metadata tests do not protect final user-facing copy; helper-level string tests are needed when wording carries behavioral meaning.
[pattern] Manual runtime plans should include non-destructive fixture setup and interaction/inspection prerequisites; otherwise repeated runs can document the same blocker without increasing confidence.
[learning] Debug inspection output is only useful evidence when it is emitted from the same rendered branch as the UI; pre-dispatch snapshots can contradict loading placeholders.
[anti-pattern] Duplicating presentation formatters in debug inspection creates false evidence; structured labels must share the same display formatter as rendered widgets.
[pattern] Keep debug inspection DTOs raw where possible and derive display labels at the UI/serialization boundary with the same helpers used by rendered widgets.
[learning] Repeating blocked runtime evidence runs without first restoring OBS prerequisites and a UI control path only adds log volume; convert the blocker into an executable-environment task.
[anti-pattern] Command-scoped failures should not reuse connection-level error events; generic session error handlers can reset UI state and make localized operation failures look like disconnections.
[learning] Localized async command failures still need an explicit non-transitioning recovery state when follow-up status refresh fails; otherwise pending UI can remain disabled without a connection error.
[pattern] Command-failure recovery events should carry an explicit reducer-visible fallback state so best-effort status refresh failures cannot strand controls in synthetic pending states.
[learning] Naming command recovery state as fallback status protects future code from treating a local UI unblocking value as an authoritative OBS status read.
[pattern] Compact operational error UI should show stable, user-facing failure labels while preserving raw backend detail in a tooltip or details affordance.
[learning] Moving a command contract type out of reducer state is incomplete if reducer helpers still construct command-recovery payloads from current state.
[learning] Public event constructors can preserve orchestration leakage even after reducer helpers are removed; narrow constructors as well as state APIs.
[pattern] Event payloads with reducer-visible invariants need private fields or invariant-preserving constructors; otherwise direct struct literals bypass the boundary.
[learning] GTK layout stability is not proven by CSS width hints or helper-string tests; long operational text needs widget-level bounds plus render evidence.
