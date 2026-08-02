/* ============================================================================
   Entry point.
   ============================================================================
   Order matters here. Everything that affects what the visitor sees and can
   operate runs immediately; the two renderers are deferred until after the
   load event and an idle slot, so ~266 KB of WebGL never competes with the
   hero for bandwidth or main thread.
   ========================================================================== */

import { initTheme, initScroll, initPointer, initReveals, initCopy, initTabs, initLightbox, refreshRelease } from "./ui.js";
import { initLive, initMixer, initStats, initGraph, initDeck } from "./demos.js";

initTheme();
initScroll();
initPointer();
initCopy();
initTabs();
initLightbox();

initDeck();
initLive();
initMixer();
initStats();
initGraph();

// Reveals are wired after the demos so the nodes they create are observed too.
initReveals();

const idle = window.requestIdleCallback || ((fn) => setTimeout(fn, 200));

// The deploy already stamped the current release into the markup; this only
// catches the window between a release and the next deploy. It is strictly a
// nice-to-have, so it waits for an idle slot rather than opening a
// third-party connection while the hero is still painting.
idle(() => refreshRelease());

const bootBackground = () => {
  idle(() => {
    import("./background.js")
      .then((module) => module.initBackground())
      .catch((error) => console.warn("[scenedeck] background unavailable:", error));
  });
};

if (document.readyState === "complete") bootBackground();
else window.addEventListener("load", bootBackground, { once: true });
