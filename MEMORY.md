[pattern] Keep async UI loading state owned by controller events or a reducer; UI pre-marking plus controller events makes stale responses harder to reason about.
[pattern] For operational controls, extract pure decision helpers around safety behavior so defaults and edge cases can be unit-tested outside GTK callbacks.
[learning] Debounced sliders still need immediate local feedback for all volume controls; otherwise non-slider controls appear inert while waiting for OBS reconciliation.
