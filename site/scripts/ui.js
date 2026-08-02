/* ============================================================================
   Page chrome: colour scheme, scroll progress, reveals, copy, tabs, toast.
   ========================================================================== */

export const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

/** Shared scroll/pointer snapshot. Written once per frame, read by everything
 *  else — so nothing else is allowed to touch scrollY during a frame. */
export const frameState = {
  progress: 0,
  pointerX: 0,
  pointerY: 0,
};

/* ---- Colour scheme ------------------------------------------------------- */

export function initTheme() {
  const root = document.documentElement;
  const toggle = document.getElementById("theme-toggle");
  const system = window.matchMedia("(prefers-color-scheme: dark)");

  const isDark = () => root.dataset.theme === "dark";

  const sync = () => {
    toggle.setAttribute("aria-pressed", String(isDark()));
    toggle.setAttribute(
      "aria-label",
      isDark() ? "Switch to the light colour scheme" : "Switch to the dark colour scheme"
    );
  };

  const apply = (dark) => {
    if (dark) root.dataset.theme = "dark";
    else root.dataset.theme = "light";
    sync();
    window.dispatchEvent(new CustomEvent("scenedeck:theme", { detail: { dark } }));
  };

  toggle.addEventListener("click", () => {
    const next = !isDark();
    try {
      localStorage.setItem("scenedeck-theme", next ? "dark" : "light");
    } catch (e) {
      /* Storage blocked; the choice just will not persist. */
    }

    // The View Transition is pure decoration — if the browser lacks it, or the
    // visitor asked for less motion, the swap simply happens.
    if (document.startViewTransition && !reducedMotion.matches) {
      document.startViewTransition(() => apply(next));
    } else {
      apply(next);
    }
  });

  // Follow the system only while the visitor has not made an explicit choice.
  system.addEventListener("change", (event) => {
    let stored = null;
    try {
      stored = localStorage.getItem("scenedeck-theme");
    } catch (e) {
      /* ignore */
    }
    if (!stored) apply(event.matches);
  });

  sync();
}

/* ---- Scroll progress ----------------------------------------------------- */
/* One rAF-throttled scroll handler for the whole page. It reads at the top of
   the frame and writes only custom properties, so nothing here can trigger a
   synchronous layout in the middle of a frame. */

export function initScroll() {
  const root = document.documentElement;
  const rail = document.getElementById("rail");
  let queued = false;

  const measure = () => {
    queued = false;
    const max = root.scrollHeight - window.innerHeight;
    const progress = max > 0 ? Math.min(1, Math.max(0, window.scrollY / max)) : 0;
    frameState.progress = progress;
    root.style.setProperty("--sp", progress.toFixed(4));
    rail.toggleAttribute("data-scrolled", window.scrollY > 40);
  };

  const onScroll = () => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(measure);
  };

  window.addEventListener("scroll", onScroll, { passive: true });
  window.addEventListener("resize", onScroll, { passive: true });
  measure();
}

/* ---- Pointer ------------------------------------------------------------- */
/* A single passive listener that only records values. The background samples
   them once per frame rather than reacting per event. */

export function initPointer() {
  window.addEventListener(
    "pointermove",
    (event) => {
      frameState.pointerX = (event.clientX / window.innerWidth) * 2 - 1;
      frameState.pointerY = (event.clientY / window.innerHeight) * 2 - 1;
    },
    { passive: true }
  );
}

/* ---- Reveals ------------------------------------------------------------- */

export function initReveals() {
  const root = document.documentElement;
  const targets = [...document.querySelectorAll("[data-reveal]")];

  if (reducedMotion.matches || !("IntersectionObserver" in window)) return;

  // Opting in here is what arms the CSS. Until this attribute exists every
  // target renders normally, so the page is never blank waiting on a script.
  root.setAttribute("data-reveal-active", "");

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        entry.target.setAttribute("data-shown", "");
        observer.unobserve(entry.target);
      }
    },
    { rootMargin: "-8% 0px -12% 0px", threshold: 0 }
  );

  targets.forEach((el) => observer.observe(el));

  // Watchdog. If the observer has not reported anything a second in — a
  // starved main thread, a browser quirk — drop the gate entirely rather than
  // leave the page hidden. Correctness of content beats the animation.
  setTimeout(() => {
    if (!document.querySelector("[data-reveal][data-shown]")) {
      observer.disconnect();
      root.removeAttribute("data-reveal-active");
    }
  }, 1000);
}

/* ---- Toast --------------------------------------------------------------- */

let toastTimer = 0;

export function toast(message) {
  const el = document.getElementById("toast");
  el.textContent = message;
  el.setAttribute("data-shown", "");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.removeAttribute("data-shown"), 2000);
}

/* ---- Copy buttons -------------------------------------------------------- */

export function initCopy() {
  document.querySelectorAll("[data-copy]").forEach((button) => {
    button.addEventListener("click", async () => {
      const text = button.getAttribute("data-copy");
      try {
        await navigator.clipboard.writeText(text);
      } catch (e) {
        // Clipboard access can be refused (insecure context, permissions).
        // Say so rather than showing a success state that did not happen.
        toast("Copy blocked — select the command instead");
        return;
      }
      button.setAttribute("data-copied", "true");
      toast("Copied to clipboard");
      setTimeout(() => button.removeAttribute("data-copied"), 1600);
    });
  });
}

/* ---- Install tabs -------------------------------------------------------- */
/* Full APG tab semantics: roving tabindex, arrow/Home/End keys, and the
   selected pane is the only one not hidden. */

export function initTabs() {
  const list = document.querySelector(".patchbay__tabs");
  if (!list) return;

  const tabs = [...list.querySelectorAll('[role="tab"]')];

  const select = (tab, { focus = true } = {}) => {
    for (const other of tabs) {
      const selected = other === tab;
      other.setAttribute("aria-selected", String(selected));
      other.tabIndex = selected ? 0 : -1;
      document.getElementById(other.getAttribute("aria-controls")).hidden = !selected;
    }
    if (focus) tab.focus();
  };

  list.addEventListener("click", (event) => {
    const tab = event.target.closest('[role="tab"]');
    if (tab) select(tab);
  });

  list.addEventListener("keydown", (event) => {
    const index = tabs.indexOf(document.activeElement);
    if (index < 0) return;

    const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
    let next = null;

    if (step) next = tabs[(index + step + tabs.length) % tabs.length];
    else if (event.key === "Home") next = tabs[0];
    else if (event.key === "End") next = tabs[tabs.length - 1];

    if (next) {
      event.preventDefault();
      select(next);
    }
  });
}

/* ---- Release stamping ---------------------------------------------------- */
/* The markup already carries working links for the release that was current
   when the page was built, and the deploy rewrites them. This is a third
   layer: if the API answers, the page matches the newest release even between
   deploys. If it does not, nothing changes and every link still works. */

export async function refreshRelease() {
  let version;
  try {
    const response = await fetch("https://api.github.com/repos/worxbend/scenedeck/releases/latest", {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!response.ok) return;
    const data = await response.json();
    version = String(data.tag_name || "").replace(/^v/, "");
  } catch (e) {
    return; // Offline, rate-limited, or blocked. The baked-in version stands.
  }

  if (!/^\d+\.\d+\.\d+$/.test(version)) return;

  document.querySelectorAll("[data-release-version]").forEach((el) => {
    el.textContent = `v${version}`;
  });

  const swap = (value) => value.replace(/scenedeck-\d+\.\d+\.\d+-linux/g, `scenedeck-${version}-linux`);

  document.querySelectorAll('a[href*="scenedeck-"]').forEach((a) => {
    a.href = swap(a.href);
  });
  document.querySelectorAll("[data-copy]").forEach((el) => {
    el.setAttribute("data-copy", swap(el.getAttribute("data-copy")));
  });
  // Only the text nodes are rewritten. Replacing textContent wholesale would
  // flatten the <span class="prompt">$</span> that colours the shell prompt.
  document.querySelectorAll(".pane code, .hero__code code").forEach((el) => {
    for (const node of el.childNodes) {
      if (node.nodeType === Node.TEXT_NODE && node.nodeValue.includes("scenedeck-")) {
        node.nodeValue = swap(node.nodeValue);
      }
    }
  });
}

/* ---- Screenshot viewer --------------------------------------------------- */
/* Built on <dialog>, which brings modal semantics, the focus trap, Escape
   handling and inert-ing the rest of the page with it. Reimplementing any of
   that by hand is how these end up unusable with a keyboard. */

export function initLightbox() {
  const dialog = document.getElementById("lightbox");
  const triggers = [...document.querySelectorAll(".shot__open")];
  if (!dialog || !triggers.length || !dialog.showModal) return;

  const image = document.getElementById("lightbox-img");
  const caption = document.getElementById("lightbox-cap");
  const count = document.getElementById("lightbox-count");

  const shots = triggers.map((button) => {
    const img = button.querySelector("img");
    return {
      button,
      src: img.getAttribute("src"),
      alt: img.getAttribute("alt"),
      caption: button.closest("figure").querySelector("figcaption").textContent.trim(),
    };
  });

  let index = 0;
  let opener = null;

  const show = (next) => {
    index = (next + shots.length) % shots.length;
    const shot = shots[index];
    image.src = shot.src;
    image.alt = shot.alt;
    caption.textContent = shot.caption;
    count.textContent = `${index + 1} of ${shots.length}`;
  };

  const open = (from) => {
    opener = triggers[from];
    show(from);
    dialog.showModal();
  };

  triggers.forEach((button, i) => button.addEventListener("click", () => open(i)));

  dialog.addEventListener("click", (event) => {
    if (event.target.closest("[data-step]")) {
      show(index + Number(event.target.closest("[data-step]").dataset.step));
      return;
    }
    // A click that lands on the dialog itself is a click on the backdrop: the
    // frame and the controls are children, so they never match.
    if (event.target === dialog || event.target.closest("[data-close]")) dialog.close();
  });

  dialog.addEventListener("keydown", (event) => {
    if (event.key === "ArrowRight") {
      event.preventDefault();
      show(index + 1);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      show(index - 1);
    }
  });

  // Returning focus to a thumbnail, rather than to the top of the page, is the
  // difference between usable and merely accessible. It goes to whichever shot
  // is on screen when the viewer closes, which may not be the one that opened
  // it. Deferred a frame so it wins over the dialog's own restoration.
  dialog.addEventListener("close", () => {
    const target = shots[index].button || opener;
    target.focus();
    // Again next frame, because the dialog's own focus restoration runs after
    // the close event and would otherwise put focus back where it was.
    requestAnimationFrame(() => target.focus());
  });
}
