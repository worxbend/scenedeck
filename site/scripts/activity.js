// Retain intersection state: showing a tab need not trigger another observer
// entry. Every decorative loop reconciles the same visibility/motion policy.
export const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

export function watchActivity({ element, change, requireFocus = false }) {
  let intersecting = !element || !("IntersectionObserver" in window);
  let focused = requireFocus ? document.hasFocus() : true;
  const reconcile = () => change(intersecting && !document.hidden && !reducedMotion.matches && focused);
  const focus = () => { focused = true; reconcile(); };
  const blur = () => { focused = false; reconcile(); };
  let observer;
  if (element && "IntersectionObserver" in window) {
    observer = new IntersectionObserver(([entry]) => {
      intersecting = entry.isIntersecting;
      reconcile();
    });
    observer.observe(element);
  }
  document.addEventListener("visibilitychange", reconcile);
  reducedMotion.addEventListener("change", reconcile);
  if (requireFocus) {
    window.addEventListener("focus", focus);
    window.addEventListener("blur", blur);
  }
  reconcile();
  return () => {
    observer?.disconnect();
    document.removeEventListener("visibilitychange", reconcile);
    reducedMotion.removeEventListener("change", reconcile);
    window.removeEventListener("focus", focus);
    window.removeEventListener("blur", blur);
    change(false);
  };
}

export function whileVisible(element, ms, tick) {
  let timer = 0;
  return watchActivity({ element, change: (active) => {
    if (active && !timer) timer = setInterval(tick, ms);
    else if (!active && timer) { clearInterval(timer); timer = 0; }
  } });
}
