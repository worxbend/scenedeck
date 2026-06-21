[pattern] Keep async UI loading state owned by controller events or a reducer; UI pre-marking plus controller events makes stale responses harder to reason about.
[pattern] For operational controls, extract pure decision helpers around safety behavior so defaults and edge cases can be unit-tested outside GTK callbacks.
[learning] Debounced sliders still need immediate local feedback for all volume controls; otherwise non-slider controls appear inert while waiting for OBS reconciliation.
[anti-pattern] Reusing one duplicate-dispatch guard for automatic rebuilds and explicit user actions can accidentally block retries after recoverable failures.
[anti-pattern] Public mirror fields around reducer-owned state drift unless every event path updates the reducer snapshot and the mirror together.
[pattern] Make reducer-owned mirror fields private once accessors exist; compiler-enforced boundaries catch future UI bypasses better than comments.
[learning] UI reconciliation predicates must cover every render source for a page mode; fixing reducer-backed modes can still leave direct-state modes visually stale.
[pattern] A single visible render-source helper keeps Mixer rendering decisions explicit across modes, but event predicates must consume the same helper to avoid duplicated fallback logic.
