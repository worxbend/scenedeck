/* ============================================================================
   The app recreations.
   ============================================================================
   Every interactive panel on the page is a rebuild of a real SceneDeck screen,
   not a screenshot: it themes with the site, stays sharp at any zoom, and can
   be operated with a keyboard. The data is representative, and the page says
   so wherever a number appears.
   ========================================================================== */

import { hexToOklch, oklchToHex, contrast } from "./color.js";
import { toast, reducedMotion } from "./ui.js";

/* ---- Fixtures ------------------------------------------------------------ */
/* A believable mid-size collection: six primary scenes plus the utility scenes
   that motivate the whole role system. */

const SCENES = [
  { name: "Main Cam", accent: "--acc-indigo" },
  { name: "Code + Cam", accent: "--acc-teal" },
  { name: "Full Screen Share", accent: "--acc-blue" },
  { name: "Game Capture", accent: "--acc-purple" },
  { name: "BRB", accent: "--acc-orange" },
  { name: "Starting Soon", accent: "--acc-green" },
];

const CHANNELS = [
  { name: "Desktop Audio", level: 0.72, muted: false, locked: false },
  { name: "Mic/Aux", level: 0.84, muted: false, locked: true },
  { name: "Game", level: 0.61, muted: false, locked: false },
  { name: "Music", level: 0.38, muted: true, locked: false },
  { name: "Discord", level: 0.55, muted: false, locked: false },
];

const ACCENTS = [
  { token: "--acc-indigo", label: "Indigo" },
  { token: "--acc-purple", label: "Purple" },
  { token: "--acc-blue", label: "Blue" },
  { token: "--acc-teal", label: "Teal" },
  { token: "--acc-green", label: "Green" },
  { token: "--acc-orange", label: "Orange" },
  { token: "--acc-red", label: "Red" },
];

const cssVar = (name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

/* ---- Dynamic colour ------------------------------------------------------ */
/* Picking a scene accent re-derives the whole primary tonal family from that
   one hue and writes it back as custom properties — which is what Material You
   does, done in the open. The WebGL layer reads the same hue, so the metal and
   the signal retint with the page in one motion. */

export function applyAccent(hex) {
  const root = document.documentElement;
  const { C, H } = hexToOklch(hex);
  const chroma = Math.min(C, 0.2);
  const dark = root.dataset.theme === "dark";

  const tone = (L, scale = 1) => oklchToHex(L, chroma * scale, H);

  const roles = dark
    ? {
        "--md-primary": tone(0.82),
        "--md-on-primary": tone(0.26),
        "--md-primary-container": tone(0.4),
        "--md-on-primary-container": tone(0.9, 0.9),
      }
    : {
        "--md-primary": tone(0.48),
        "--md-on-primary": "#ffffff",
        "--md-primary-container": tone(0.92, 0.55),
        "--md-on-primary-container": tone(0.22),
      };

  // The palette is generated, so it gets checked rather than trusted. If a hue
  // lands short of AA against the surface it is darkened (light) or lightened
  // (dark) until it clears — the accent still reads, the text stays legible.
  const surface = cssVar("--md-surface") || (dark ? "#0e0c13" : "#faf9f8");
  let L = dark ? 0.82 : 0.48;
  while (contrast(roles["--md-primary"], surface) < 4.5 && L > 0.1 && L < 0.98) {
    L += dark ? 0.02 : -0.02;
    roles["--md-primary"] = tone(L);
  }

  for (const [name, value] of Object.entries(roles)) root.style.setProperty(name, value);
  root.style.setProperty("--src-h", H.toFixed(2));
  root.style.setProperty("--src-c", chroma.toFixed(4));

  window.dispatchEvent(new CustomEvent("scenedeck:accent", { detail: { hex, hue: H, chroma } }));
}

/* ---- Live page ----------------------------------------------------------- */

export function initLive() {
  const cards = document.getElementById("live-cards");
  const chips = document.getElementById("accent-chips");
  const program = document.getElementById("live-program");
  if (!cards) return;

  SCENES.forEach((scene, index) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "scene-card";
    button.style.setProperty("--scene-accent", cssVar(scene.accent));
    button.setAttribute("aria-pressed", String(index === 0));
    if (index === 0) button.setAttribute("data-program", "");
    button.innerHTML = `
      <span class="scene-card__tally"><span class="pip pip--live"></span>PGM</span>
      <span class="scene-card__name">${scene.name}</span>
      <span class="legend">Primary</span>`;

    button.addEventListener("click", () => {
      cards.querySelectorAll(".scene-card").forEach((card) => {
        card.removeAttribute("data-program");
        card.setAttribute("aria-pressed", "false");
      });
      button.setAttribute("data-program", "");
      button.setAttribute("aria-pressed", "true");
      program.textContent = scene.name;

      // A program take is the one moment the background reacts to the page:
      // the tally lamps run across the panel wall.
      window.dispatchEvent(new CustomEvent("scenedeck:take", { detail: { index } }));
    });

    cards.append(button);
  });

  ACCENTS.forEach((accent, index) => {
    const hex = cssVar(accent.token);
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = "chip";
    chip.setAttribute("aria-pressed", String(index === 0));
    chip.innerHTML = `<span class="chip__swatch" style="--swatch:${hex}"></span>${accent.label}`;

    chip.addEventListener("click", () => {
      chips.querySelectorAll(".chip").forEach((c) => c.setAttribute("aria-pressed", "false"));
      chip.setAttribute("aria-pressed", "true");
      applyAccent(hex);
    });

    chips.append(chip);
  });
}

/* ---- Mixer --------------------------------------------------------------- */
/* The faders are real sliders: pointer-draggable, and operable with the arrow
   keys, Home, End and Page Up/Down. Locking one takes it out of the tab order
   and stops it responding — which is exactly what the lock does in the app. */

export function initMixer() {
  const mixer = document.getElementById("mixer");
  if (!mixer) return;

  const strips = CHANNELS.map((channel) => {
    const el = document.createElement("div");
    el.className = "strip";

    const meter = document.createElement("div");
    meter.className = "meter strip__meter";
    const segments = Array.from({ length: 12 }, () => {
      const seg = document.createElement("span");
      seg.className = "meter__seg";
      meter.append(seg);
      return seg;
    });

    const fader = document.createElement("div");
    fader.className = "fader";
    fader.tabIndex = 0;
    fader.setAttribute("role", "slider");
    fader.setAttribute("aria-orientation", "vertical");
    fader.setAttribute("aria-label", `${channel.name} level`);
    fader.setAttribute("aria-valuemin", "0");
    fader.setAttribute("aria-valuemax", "100");
    fader.innerHTML = '<span class="fader__cap"></span>';

    const stage = document.createElement("div");
    stage.className = "strip__stage";
    stage.append(meter, fader);

    const readout = document.createElement("span");
    readout.className = "strip__readout";

    const mute = document.createElement("button");
    mute.type = "button";
    mute.className = "strip__btn";
    mute.textContent = "M";
    mute.setAttribute("aria-label", `Mute ${channel.name}`);

    const lock = document.createElement("button");
    lock.type = "button";
    lock.className = "strip__btn strip__lock";
    lock.textContent = "L";
    lock.setAttribute("aria-label", `Lock the ${channel.name} slider`);

    const controls = document.createElement("div");
    controls.className = "strip__controls";
    controls.append(mute, lock);

    const name = document.createElement("span");
    name.className = "strip__name";
    name.textContent = channel.name;

    el.append(name, stage, readout, controls);
    mixer.append(el);

    const state = { ...channel, peak: channel.level };

    const render = () => {
      fader.style.setProperty("--level", state.level.toFixed(3));
      fader.setAttribute("aria-valuenow", Math.round(state.level * 100));
      // OBS reports volume in dB; -inf at the bottom of the throw.
      const db = state.level <= 0.001 ? "-inf" : (Math.log10(state.level) * 20).toFixed(1);
      readout.textContent = state.muted ? "muted" : `${db} dB`;
      fader.setAttribute("aria-valuetext", state.muted ? "Muted" : `${db} decibels`);
      fader.dataset.locked = String(state.locked);
      fader.setAttribute("aria-disabled", String(state.locked));
      fader.tabIndex = state.locked ? -1 : 0;
      mute.setAttribute("aria-pressed", String(state.muted));
      lock.setAttribute("aria-pressed", String(state.locked));
    };

    const setLevel = (next) => {
      if (state.locked) return;
      state.level = Math.min(1, Math.max(0, next));
      render();
    };

    // Pointer drag. Capture keeps the gesture alive if the pointer leaves the
    // 34px-wide track, which it will.
    fader.addEventListener("pointerdown", (event) => {
      if (state.locked) return;
      fader.setPointerCapture(event.pointerId);
      const track = () => {
        const rect = fader.getBoundingClientRect();
        return rect;
      };
      const move = (e) => {
        const rect = track();
        setLevel(1 - (e.clientY - rect.top) / rect.height);
      };
      move(event);
      fader.addEventListener("pointermove", move);

      const end = () => {
        fader.removeEventListener("pointermove", move);
        fader.removeEventListener("pointerup", end);
        fader.removeEventListener("pointercancel", end);
        if (fader.hasPointerCapture(event.pointerId)) fader.releasePointerCapture(event.pointerId);
      };
      fader.addEventListener("pointerup", end);
      fader.addEventListener("pointercancel", end);
    });

    fader.addEventListener("keydown", (event) => {
      const steps = {
        ArrowUp: 0.02,
        ArrowRight: 0.02,
        ArrowDown: -0.02,
        ArrowLeft: -0.02,
        PageUp: 0.1,
        PageDown: -0.1,
      };
      if (event.key in steps) {
        event.preventDefault();
        setLevel(state.level + steps[event.key]);
      } else if (event.key === "Home") {
        event.preventDefault();
        setLevel(0);
      } else if (event.key === "End") {
        event.preventDefault();
        setLevel(1);
      }
    });

    mute.addEventListener("click", () => {
      state.muted = !state.muted;
      render();
    });

    lock.addEventListener("click", () => {
      state.locked = !state.locked;
      render();
      toast(state.locked ? `${channel.name} slider locked` : `${channel.name} slider unlocked`);
    });

    render();
    return { state, segments };
  });

  // The meters. Attack fast, release slow — the asymmetry is what makes it
  // read as audio instead of as a sine wave.
  if (reducedMotion.matches) {
    strips.forEach(({ state, segments }) => paintMeter(segments, state.muted ? 0 : state.level * 0.8));
    return;
  }

  let t = 0;
  whileVisible(mixer, 100, () => {
    t += 0.1;
    strips.forEach(({ state, segments }, i) => {
      const envelope =
        0.5 + 0.28 * Math.sin(t * 1.7 + i * 0.41) + 0.14 * Math.sin(t * 4.3 + i * 1.13);
      const target = state.muted ? 0 : Math.max(0, envelope) * state.level;
      const rate = target > state.peak ? 0.55 : 0.12;
      state.peak += (target - state.peak) * rate;
      paintMeter(segments, state.peak);
    });
  });
}

/** Run `tick` on an interval, but only while `el` is on screen and the tab is
 *  visible. A demo that animates in a scrolled-past section is pure waste. */
function whileVisible(el, ms, tick) {
  let timer = 0;

  const start = () => {
    if (!timer) timer = setInterval(tick, ms);
  };
  const stop = () => {
    clearInterval(timer);
    timer = 0;
  };

  const observer = new IntersectionObserver(([entry]) => {
    if (entry.isIntersecting && !document.hidden) start();
    else stop();
  });
  observer.observe(el);

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) stop();
  });
}

function paintMeter(segments, level) {
  const lit = Math.round(level * segments.length);
  segments.forEach((seg, i) => {
    seg.toggleAttribute("data-on", i < lit);
    seg.toggleAttribute("data-hot", i >= segments.length - 4 && i < segments.length - 1);
    seg.toggleAttribute("data-peak", i === segments.length - 1);
  });
}

/* ---- Stats --------------------------------------------------------------- */

const GAUGES = [
  { id: "fps", label: "FPS", max: 60, value: 60, decimals: 1, unit: "of 60" },
  { id: "frame", label: "Avg frame render", max: 16.6, value: 3.4, decimals: 2, unit: "ms" },
  { id: "congestion", label: "Congestion", max: 1, value: 0.02, decimals: 2, unit: "0–1" },
];

const COUNTERS = [
  { label: "Render frames missed", value: 0 },
  { label: "Render frames skipped", value: 2 },
  { label: "Output frames missed", value: 0 },
  { label: "Output frames skipped", value: 0 },
];

export function initStats() {
  const wrap = document.getElementById("gauges");
  if (!wrap) return;

  // A 220-degree arc, drawn once and animated through stroke-dashoffset.
  const R = 70;
  const SWEEP = 220;
  const circumference = 2 * Math.PI * R;
  const arc = (circumference * SWEEP) / 360;

  const gauges = GAUGES.map((g) => {
    const el = document.createElement("div");
    el.className = "gauge";
    el.innerHTML = `
      <svg viewBox="0 0 180 140" role="img" aria-label="${g.label}">
        <path class="gauge__track" d="${arcPath(90, 92, R, SWEEP)}"
              stroke-dasharray="${arc} ${circumference}" />
        <path class="gauge__value" d="${arcPath(90, 92, R, SWEEP)}"
              stroke-dasharray="${arc} ${circumference}" stroke-dashoffset="${arc}" />
      </svg>
      <span class="gauge__readout" data-readout>0</span>
      <span class="legend">${g.label}</span>
      <span class="gauge__unit">${g.unit}</span>`;
    wrap.append(el);
    return { ...g, el, value: g.value, path: el.querySelector(".gauge__value"), readout: el.querySelector("[data-readout]") };
  });

  const counters = document.getElementById("counters");
  COUNTERS.forEach((c) => {
    const el = document.createElement("div");
    el.className = "counter";
    el.innerHTML = `<span class="counter__value">${c.value}</span><span class="legend">${c.label}</span>`;
    counters.append(el);
  });

  const paintGauge = (g) => {
    const fraction = Math.min(1, Math.max(0, g.value / g.max));
    g.path.setAttribute("stroke-dashoffset", String(arc * (1 - fraction)));
    g.readout.textContent = g.value.toFixed(g.decimals);

    // Severity thresholds mirror the app: FPS is bad when it falls, the other
    // two are bad when they rise.
    let severity = "ok";
    if (g.id === "fps") severity = fraction < 0.75 ? "error" : fraction < 0.92 ? "warn" : "ok";
    else severity = fraction > 0.75 ? "error" : fraction > 0.5 ? "warn" : "ok";
    g.el.dataset.severity = severity;
  };

  const bitrate = seededSeries(48, 6000, 260);
  const frame = seededSeries(48, 3.4, 0.55);
  const bitrateNow = document.getElementById("chart-bitrate-now");
  const frameNow = document.getElementById("chart-frame-now");

  const paintCharts = () => {
    drawSeries("chart-bitrate", "chart-bitrate-area", bitrate);
    drawSeries("chart-frame", "chart-frame-area", frame);
    bitrateNow.textContent = Math.round(bitrate[bitrate.length - 1]);
    frameNow.textContent = frame[frame.length - 1].toFixed(1);
  };

  gauges.forEach(paintGauge);
  paintCharts();

  if (reducedMotion.matches) return;

  // The Stats page polls OBS once a second, so the demo advances once a second
  // too — and it only runs while the section is actually on screen.
  let timer = 0;
  const observer = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting && !timer) {
        timer = setInterval(() => {
          gauges.forEach((g) => {
            const drift = { fps: 0.35, frame: 0.22, congestion: 0.012 }[g.id];
            g.value = clampTo(g, g.value + (Math.random() - 0.5) * drift * 2);
            paintGauge(g);
          });
          bitrate.push(clamp(bitrate[bitrate.length - 1] + (Math.random() - 0.5) * 420, 4800, 6800));
          bitrate.shift();
          frame.push(clamp(frame[frame.length - 1] + (Math.random() - 0.5) * 0.7, 1.8, 7.5));
          frame.shift();
          paintCharts();
        }, 1000);
      } else if (!entry.isIntersecting && timer) {
        clearInterval(timer);
        timer = 0;
      }
    },
    { threshold: 0 }
  );
  observer.observe(document.getElementById("stats"));
}

const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

function clampTo(g, v) {
  if (g.id === "fps") return clamp(v, 54, 60);
  if (g.id === "frame") return clamp(v, 1.8, 7.5);
  return clamp(v, 0, 0.14);
}

function arcPath(cx, cy, r, sweep) {
  const start = 180 + (180 - sweep) / 2;
  const end = start + sweep;
  const p = (deg) => [cx + r * Math.cos((deg * Math.PI) / 180), cy + r * Math.sin((deg * Math.PI) / 180)];
  const [x1, y1] = p(start);
  const [x2, y2] = p(end);
  return `M ${x1.toFixed(2)} ${y1.toFixed(2)} A ${r} ${r} 0 ${sweep > 180 ? 1 : 0} 1 ${x2.toFixed(2)} ${y2.toFixed(2)}`;
}

/** A deterministic wander, so the first paint is identical on every load. */
function seededSeries(n, base, spread) {
  const out = [];
  let v = base;
  for (let i = 0; i < n; i++) {
    v += (Math.sin(i * 1.7) + Math.sin(i * 0.43) * 0.6) * spread * 0.25;
    out.push(v);
  }
  return out;
}

function drawSeries(lineId, areaId, values) {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;
  const step = 320 / (values.length - 1);

  const points = values.map((v, i) => [i * step, 92 - ((v - min) / range) * 84]);
  const d = points.map(([x, y], i) => `${i ? "L" : "M"} ${x.toFixed(1)} ${y.toFixed(1)}`).join(" ");

  document.getElementById(lineId).setAttribute("d", d);
  document.getElementById(areaId).setAttribute("d", `${d} L 320 96 L 0 96 Z`);
}

/* ---- Graph --------------------------------------------------------------- */
/* A small, honest slice of a dependency graph: primary scenes on the left,
   the modules and source wrappers they nest on the right. */

const GRAPH_NODES = [
  { id: "main", label: "Main Cam", role: "primary", x: 12, y: 20 },
  { id: "code", label: "Code + Cam", role: "primary", x: 12, y: 92 },
  { id: "game", label: "Game Capture", role: "primary", x: 12, y: 164 },
  { id: "lower", label: "overlay-lower3", role: "module", x: 232, y: 8 },
  { id: "camraw", label: "cam-raw", role: "raw", x: 232, y: 68 },
  { id: "alerts", label: "alerts-stack", role: "module", x: 232, y: 128 },
  { id: "dbg", label: "test-scene-2", role: "debug", x: 232, y: 184 },
];

const GRAPH_EDGES = [
  ["main", "lower", "ok"],
  ["main", "camraw", "ok"],
  ["code", "camraw", "ok"],
  ["code", "alerts", "ok"],
  ["game", "alerts", "ok"],
  ["game", "dbg", "warn"],
];

export function initGraph() {
  const nodes = document.getElementById("graph-nodes");
  const edges = document.getElementById("graph-edges");
  if (!nodes) return;

  const NS = "http://www.w3.org/2000/svg";
  const byId = Object.fromEntries(GRAPH_NODES.map((n) => [n.id, n]));
  const W = 150;
  const H = 28;

  for (const [from, to, status] of GRAPH_EDGES) {
    const a = byId[from];
    const b = byId[to];
    const x1 = a.x + W;
    const y1 = a.y + H / 2;
    const x2 = b.x;
    const y2 = b.y + H / 2;
    const mid = (x1 + x2) / 2;

    const path = document.createElementNS(NS, "path");
    path.setAttribute("d", `M ${x1} ${y1} C ${mid} ${y1}, ${mid} ${y2}, ${x2} ${y2}`);
    path.dataset.status = status;
    edges.append(path);
  }

  for (const node of GRAPH_NODES) {
    const g = document.createElementNS(NS, "g");
    const rect = document.createElementNS(NS, "rect");
    rect.setAttribute("x", node.x);
    rect.setAttribute("y", node.y);
    rect.setAttribute("width", W);
    rect.setAttribute("height", H);
    rect.setAttribute("rx", "6");
    rect.dataset.role = node.role;

    const text = document.createElementNS(NS, "text");
    text.setAttribute("x", node.x + 10);
    text.setAttribute("y", node.y + 18);
    text.textContent = node.label;

    g.append(rect, text);
    nodes.append(g);
  }
}

/* ---- Deck screens -------------------------------------------------------- */
/* The hero device switches between the app's six pages. Each screen is a
   compact version of the corresponding section further down the page. */

const SCREENS = {
  live: () => `
    <p class="screen-label">Scenes · Primary</p>
    <div class="deck-scenes">
      ${SCENES.map(
        (s, i) => `<div class="deck-scene" style="--scene-accent:${cssVar(s.accent)}"
          ${i === 0 ? "data-program" : ""}>
          <span class="deck-scene__pgm">PGM</span>${s.name}</div>`
      ).join("")}
    </div>
    <p class="screen-label">Audio</p>
    <div class="deck-mixer">
      ${CHANNELS.map(
        (c) => `<div class="deck-strip">
          <div class="meter deck-strip__meter" data-deck-meter>${'<span class="meter__seg"></span>'.repeat(8)}</div>
          <span class="deck-strip__name">${c.name}</span></div>`
      ).join("")}
    </div>`,

  stats: () => `
    <p class="screen-label">Live telemetry</p>
    <div class="deck-scenes" style="grid-template-columns:repeat(2,minmax(0,1fr))">
      ${[
        ["FPS", "60.0"],
        ["Frame render", "3.4 ms"],
        ["Bitrate", "6000 kb/s"],
        ["Congestion", "0.02"],
      ]
        .map(
          ([k, v]) => `<div class="deck-scene" style="--scene-accent:${cssVar("--acc-indigo")};cursor:default">
            <span style="display:block"><span class="deck-strip__name" style="text-align:left">${k}</span>
            <span class="mono" style="font-size:.95rem">${v}</span></span></div>`
        )
        .join("")}
    </div>
    <p class="screen-label">Dropped frames: 0</p>`,

  graph: () => `
    <p class="screen-label">Scene dependencies</p>
    <div class="graphviz" style="flex:1">
      <svg viewBox="0 0 420 220" role="presentation" style="height:100%">
        <g class="graphviz__edges">${GRAPH_EDGES.map(([f, t, s]) => {
          const a = GRAPH_NODES.find((n) => n.id === f);
          const b = GRAPH_NODES.find((n) => n.id === t);
          const mid = (a.x + 150 + b.x) / 2;
          return `<path d="M ${a.x + 150} ${a.y + 14} C ${mid} ${a.y + 14}, ${mid} ${b.y + 14}, ${b.x} ${b.y + 14}" data-status="${s}" />`;
        }).join("")}</g>
        <g class="graphviz__nodes">${GRAPH_NODES.map(
          (n) =>
            `<g><rect x="${n.x}" y="${n.y}" width="150" height="28" rx="6" data-role="${n.role}" /><text x="${n.x + 10}" y="${n.y + 18}">${n.label}</text></g>`
        ).join("")}</g>
      </svg>
    </div>`,

  inventory: () => `
    <p class="screen-label">Scene roles</p>
    <div style="display:flex;flex-direction:column;gap:6px">
      ${[
        ["Main Cam", "primary", "Primary"],
        ["Code + Cam", "primary", "Primary"],
        ["overlay-lower3", "module", "Module"],
        ["cam-raw", "raw", "Raw"],
        ["test-scene-2", "debug", "Debug"],
        ["old-layout-2025", "archive", "Archive"],
      ]
        .map(
          ([name, role, label]) =>
            `<div style="display:flex;align-items:center;gap:10px;padding:7px 10px;border-radius:8px;background:var(--md-surface-container-lowest);box-shadow:var(--bevel-recessed)">
              <span style="opacity:.4;cursor:grab" aria-hidden="true">⠿</span>
              <span style="font-size:.75rem;flex:1">${name}</span>
              <span class="tag tag--${role}">${label}</span></div>`
        )
        .join("")}
    </div>`,

  doctor: () => `
    <p class="screen-label">Diagnostics · 5 checks</p>
    <ul class="doctor" role="list">
      <li class="doctor__row" data-state="ok"><span class="doctor__mark"></span><span>Registry entries all resolve</span></li>
      <li class="doctor__row" data-state="warn"><span class="doctor__mark"></span><span>“Game Capture” has no assigned role</span></li>
      <li class="doctor__row" data-state="warn"><span class="doctor__mark"></span><span>Primary nests a Debug scene</span></li>
      <li class="doctor__row" data-state="error"><span class="doctor__mark"></span><span>Cycle: alerts-stack ⇄ overlay-lower3</span></li>
      <li class="doctor__row" data-state="ok"><span class="doctor__mark"></span><span>No Raw scene nests another scene</span></li>
    </ul>`,

  settings: () => `
    <p class="screen-label">OBS connection</p>
    <div style="display:flex;flex-direction:column;gap:10px">
      ${[
        ["Host", "127.0.0.1"],
        ["Port", "4455"],
        ["Password", "•••••••• (system keyring)"],
      ]
        .map(
          ([k, v]) =>
            `<label style="display:flex;flex-direction:column;gap:4px">
              <span class="screen-label">${k}</span>
              <span class="mono" style="font-size:.75rem;padding:8px 10px;border-radius:8px;background:var(--md-surface-container-lowest);box-shadow:var(--bevel-recessed)">${v}</span>
            </label>`
        )
        .join("")}
      <p class="screen-label">Appearance · Follow system · Studio Purple</p>
    </div>`,
};

export function initDeck() {
  const screen = document.getElementById("deck-screen");
  if (!screen) return;

  const pages = [...document.querySelectorAll(".deck__page")];

  const show = (name) => {
    screen.innerHTML = SCREENS[name]();
    // Illustration only — the switcher buttons above it are the real controls.
    screen.setAttribute("aria-hidden", "true");
    for (const page of pages) {
      const active = page.dataset.page === name;
      page.classList.toggle("is-active", active);
      page.setAttribute("aria-pressed", String(active));
    }
    deckGeneration += 1;
    if (name === "live") animateDeckMeters(screen, deckGeneration);
  };

  pages.forEach((page) => page.addEventListener("click", () => show(page.dataset.page)));
  show("live");

  // Status-bar numbers drift, because a status bar that never moves reads as a
  // screenshot. Linear, like the rest of the telemetry.
  if (reducedMotion.matches) return;
  const fps = document.querySelector('[data-metric="fps"]');
  const bit = document.querySelector('[data-metric="bitrate"]');
  const cpu = document.querySelector('[data-metric="cpu"]');
  whileVisible(screen, 1400, () => {
    fps.textContent = `${(59.7 + Math.random() * 0.3).toFixed(1)} fps`;
    bit.textContent = `${Math.round(5900 + Math.random() * 220)} kb/s`;
    cpu.textContent = `CPU ${(3.8 + Math.random() * 1.2).toFixed(1)}%`;
  });
}

/** Incremented on every screen swap; a running loop whose generation is stale
 *  stops on its next tick. Without it, clicking Live twice left the first loop
 *  running forever against nodes that had already been replaced. */
let deckGeneration = 0;

function animateDeckMeters(root, generation) {
  const meters = [...root.querySelectorAll("[data-deck-meter]")].map((m) => [...m.children]);
  if (!meters.length || reducedMotion.matches) return;

  let t = 0;
  const step = () => {
    if (generation !== deckGeneration || !meters[0][0].isConnected) return;
    t += 0.12;
    meters.forEach((segs, i) => {
      const level = 0.45 + 0.3 * Math.sin(t * 1.6 + i * 0.7) + 0.12 * Math.sin(t * 3.9 + i);
      paintMeter(segs, Math.max(0, level));
    });
    setTimeout(step, 110);
  };
  step();
}
