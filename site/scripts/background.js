/* ============================================================================
   The living background: the link between two machines, drawn as a board.
   ============================================================================

   Pads at the junctions, traces routed between them the way a PCB routes them
   — a straight run, a 45-degree turn, another straight run — and data moving
   along those traces. Three parallax layers give depth by moving at different
   rates rather than by projection.

   The division of labour:

   - three.js owns THE BOARD. Two instanced draw calls: one for the trace runs,
     one for the pads. Every trace is a quad stretched between two points, so
     the geometry is exact and cannot drift.
   - pixi.js owns THE TRAFFIC. Packets running the traces, and byte glyphs
     streaming along the busier ones. Thousands of individually positioned and
     tinted sprites is precisely what its batcher exists for.

   Everything is placed by net.js in one flat space with no perspective divide.
   Both renderers use the same `toScreen()`, so a packet is always exactly on
   the trace three.js drew.

   Motion is Material 3 Expressive: springs for anything physical, strictly
   linear for anything that represents data in transit.
   ========================================================================== */

import { frameState, reducedMotion } from "./ui.js";
import { SPRING, Spring } from "./spring.js";
import { net, layout, offset, tracePath, pathLength, pathPoint, toScreen } from "./net.js";

const DPR = () => Math.min(window.devicePixelRatio || 1, 2);

function hasWebGL() {
  try {
    const canvas = document.createElement("canvas");
    return Boolean(canvas.getContext("webgl2") || canvas.getContext("webgl"));
  } catch (e) {
    return false;
  }
}

/** Tier 1 drops the board and keeps the traffic: the shader is the expensive
 *  half, and the moving data is what carries the idea. */
function pickTier() {
  const w = window.innerWidth;
  let tier = w >= 1280 ? 3 : w >= 768 ? 2 : 1;
  if ((navigator.hardwareConcurrency || 8) <= 4) tier = Math.max(1, tier - 1);
  if ((navigator.deviceMemory || 8) <= 4) tier = Math.max(1, tier - 1);
  return tier;
}

const TIERS = {
  3: { board: true, nodes: 54, packets: 1400, bytes: 260, threeScale: 0.85, pixiRes: 1.5 },
  2: { board: true, nodes: 36, packets: 800, bytes: 150, threeScale: 0.75, pixiRes: 1.25 },
  1: { board: false, nodes: 24, packets: 380, bytes: 70, threeScale: 0, pixiRes: 1 },
};

const readVar = (name) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim();

const hexToInt = (hex) => Number(`0x${(hex || "#ffffff").replace("#", "")}`);

/* ---- Entry --------------------------------------------------------------- */

export async function initBackground() {
  if (!hasWebGL()) return; // The CSS fallback composition is already showing.

  const config = TIERS[pickTier()];
  layout(window.innerWidth / Math.max(1, window.innerHeight), config.nodes);

  const shared = {
    dark: document.documentElement.dataset.theme === "dark",
    burst: new Spring(0, SPRING.spatialFast),
    accent: null,
    config,
  };

  const layers = [];

  try {
    if (config.board) layers.push(await buildBoard(config, shared));
  } catch (error) {
    console.warn("[scenedeck] board layer unavailable:", error);
  }

  try {
    layers.push(await buildTraffic(config, shared));
  } catch (error) {
    console.warn("[scenedeck] traffic layer unavailable:", error);
  }

  if (!layers.length) return;

  bindAppearance(layers, shared);

  // One frame, then stop: the full composition with none of the motion.
  if (reducedMotion.matches) {
    const paint = () => layers.forEach((layer) => layer.render(0));
    paint();
    layers.forEach((layer) => layer.canvas.setAttribute("data-live", ""));
    window.addEventListener("appearance:repaint", paint);
    window.addEventListener(
      "resize",
      debounce(() => {
        layout(window.innerWidth / Math.max(1, window.innerHeight), config.nodes);
        layers.forEach((layer) => layer.resize());
        paint();
      }, 160)
    );
    return;
  }

  drive(layers, shared);
}

/** Theme and accent are page state, not animation state, so they are wired up
 *  for everyone — including the reduced-motion path. */
function bindAppearance(layers, shared) {
  const repaint = () => window.dispatchEvent(new CustomEvent("appearance:repaint"));

  window.addEventListener("scenedeck:theme", (event) => {
    shared.dark = event.detail.dark;
    for (const layer of layers) layer.theme?.(event.detail.dark);
    repaint();
  });

  window.addEventListener("scenedeck:accent", (event) => {
    shared.accent = event.detail.hex;
    for (const layer of layers) layer.accent?.(event.detail.hex);
    repaint();
  });
}

/* ---- The driver ---------------------------------------------------------- */

function drive(layers, shared) {
  let raf = 0;
  let last = 0;
  let running = false;

  const frame = (now) => {
    raf = requestAnimationFrame(frame);
    const dt = Math.min((now - last) / 1000, 0.0333);
    last = now;

    net.time += dt;
    net.scroll = frameState.progress;

    // The pointer is damped rather than sprung: it is a follow, not an event,
    // and a cursor that overshoots reads as lag.
    const k = 1 - Math.pow(1 - 0.06, dt * 60);
    net.pointerX += (frameState.pointerX - net.pointerX) * k;
    net.pointerY += (frameState.pointerY - net.pointerY) * k;

    shared.burstEnergy = shared.burst.step(dt);

    for (const layer of layers) layer.render(dt);
  };

  const start = () => {
    if (running) return;
    running = true;
    last = performance.now();
    raf = requestAnimationFrame(frame);
  };

  const stop = () => {
    if (!running) return;
    running = false;
    cancelAnimationFrame(raf);
  };

  document.addEventListener("visibilitychange", () => (document.hidden ? stop() : start()));
  window.addEventListener("blur", stop);
  window.addEventListener("focus", start);

  window.addEventListener(
    "resize",
    debounce(() => {
      layout(window.innerWidth / Math.max(1, window.innerHeight), shared.config.nodes);
      for (const layer of layers) layer.resize();
    }, 120)
  );

  // A program take pushes a burst of traffic down the board.
  window.addEventListener("scenedeck:take", () => {
    shared.burst.velocity += 9;
    shared.burst.to(1);
    clearTimeout(shared.release);
    shared.release = setTimeout(() => shared.burst.to(0), 380);
    window.dispatchEvent(new CustomEvent("net:burst"));
  });

  start();
  layers.forEach((layer) => layer.canvas.setAttribute("data-live", ""));
}

function debounce(fn, ms) {
  let timer = 0;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

/* ============================================================================
   THE BOARD — three.js
   ========================================================================== */

const COMMON = /* glsl */ `
  uniform float uAspectRatio;
  uniform float uCeil;
  uniform float uStrength;
  uniform float uInk;
  uniform vec3  uSurface;

  vec3 present(vec3 add) {
    float y = max(dot(add, vec3(0.2126, 0.7152, 0.0722)), 1e-4);
    add *= min(1.0, uCeil / y) * uStrength;

    // Dark emits over the surface; light prints the same structure as ink,
    // bounded so a border colour keeps its contrast on paper.
    float density = clamp(dot(add, vec3(0.2126, 0.7152, 0.0722)) / max(uCeil, 1e-4), 0.0, 1.0);
    vec3 lit = uSurface + add;
    vec3 ink = mix(uSurface, uSurface * 0.35, density * 0.42);
    vec3 outc = max(mix(lit, ink, uInk), 0.0);

    // three.js only injects its output encoding into its own materials, so a
    // ShaderMaterial has to encode to sRGB itself.
    return mix(outc * 12.92,
               1.055 * pow(outc, vec3(0.41666)) - 0.055,
               step(vec3(0.0031308), outc));
  }
`;

const TRACE_VERT = /* glsl */ `
  attribute vec4 aSeg;    // x0, y0, x1, y1
  attribute vec2 aMeta;   // width, dim
  varying vec2 vUv;
  varying float vDim;

  void main() {
    vUv = uv;
    vDim = aMeta.y;

    vec2 a = aSeg.xy;
    vec2 b = aSeg.zw;
    vec2 d = b - a;
    float len = max(length(d), 1e-5);
    vec2 n = vec2(-d.y, d.x) / len;

    vec2 p = a + d * uv.x + n * (uv.y - 0.5) * aMeta.x;
    gl_Position = vec4(p.x / uAspectRatio, p.y, 0.0, 1.0);
  }
`;

const TRACE_FRAG = /* glsl */ `
  precision highp float;
  varying vec2 vUv;
  varying float vDim;
  uniform vec3 uLine;

  void main() {
    // Soft across the width so a run has an edge rather than a stair.
    float cov = 1.0 - smoothstep(0.28, 0.5, abs(vUv.y - 0.5));
    vec3 col = present(uLine * 0.55 * vDim);
    gl_FragColor = vec4(col * cov, cov);
  }
`;

const PAD_VERT = /* glsl */ `
  attribute vec3 aPos;    // x, y, size
  attribute vec2 aMeta;   // dim, hub
  varying vec2 vUv;
  varying float vDim;
  varying float vHub;

  void main() {
    vUv = uv;
    vDim = aMeta.x;
    vHub = aMeta.y;
    vec2 p = position.xy * aPos.z + aPos.xy;
    gl_Position = vec4(p.x / uAspectRatio, p.y, 0.0, 1.0);
  }
`;

const PAD_FRAG = /* glsl */ `
  precision highp float;
  varying vec2 vUv;
  varying float vDim;
  varying float vHub;
  uniform vec3 uLine;
  uniform vec3 uNode;

  void main() {
    vec2 p = (vUv - 0.5) * 2.0;
    float r = max(abs(p.x), abs(p.y));   // square pads, like a real footprint

    // A solid centre, and for hubs a ring around it: a device rather than a via.
    float core = 1.0 - smoothstep(0.30, 0.40, r);
    float ring = (1.0 - smoothstep(0.80, 0.92, r)) * smoothstep(0.62, 0.74, r) * vHub;
    float cov = clamp(core + ring, 0.0, 1.0);
    if (cov < 0.004) discard;

    vec3 col = present(mix(uNode, uLine, 0.35) * (core * 1.1 + ring * 0.8) * vDim);
    gl_FragColor = vec4(col * cov, cov);
  }
`;

async function buildBoard(config, shared) {
  const THREE = await import("../vendor/three.min.js");
  const canvas = document.getElementById("bg-three");

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    depth: false,
    stencil: false,
    powerPreference: "high-performance",
  });
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.setClearAlpha(0);

  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);

  const uniforms = {
    uAspectRatio: { value: 1 },
    uCeil: { value: 0.08 },
    uStrength: { value: 0.72 },
    uInk: { value: shared.dark ? 0 : 1 },
    uSurface: { value: new THREE.Color("#080f22") },
    uLine: { value: new THREE.Color("#256eff") },
    uNode: { value: new THREE.Color("#72a2ff") },
  };

  const quad = new THREE.PlaneGeometry(1, 1);
  const clone = () => {
    const g = new THREE.InstancedBufferGeometry();
    g.index = quad.index;
    g.setAttribute("position", quad.attributes.position);
    g.setAttribute("uv", quad.attributes.uv);
    return g;
  };

  /* -- trace runs -------------------------------------------------------- */
  const SEGS = config.nodes * 3;
  const traceGeo = clone();
  const aSeg = new THREE.InstancedBufferAttribute(new Float32Array(SEGS * 4), 4);
  const aSegMeta = new THREE.InstancedBufferAttribute(new Float32Array(SEGS * 2), 2);
  traceGeo.setAttribute("aSeg", aSeg);
  traceGeo.setAttribute("aMeta", aSegMeta);

  const traceMesh = new THREE.Mesh(
    traceGeo,
    new THREE.ShaderMaterial({
      vertexShader: COMMON + TRACE_VERT,
      fragmentShader: COMMON + TRACE_FRAG,
      uniforms,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    })
  );
  traceMesh.frustumCulled = false;
  scene.add(traceMesh);

  /* -- pads -------------------------------------------------------------- */
  const padGeo = clone();
  const aPos = new THREE.InstancedBufferAttribute(new Float32Array(config.nodes * 3), 3);
  const aPadMeta = new THREE.InstancedBufferAttribute(new Float32Array(config.nodes * 2), 2);
  padGeo.setAttribute("aPos", aPos);
  padGeo.setAttribute("aMeta", aPadMeta);

  const padMesh = new THREE.Mesh(
    padGeo,
    new THREE.ShaderMaterial({
      vertexShader: COMMON + PAD_VERT,
      fragmentShader: COMMON + PAD_FRAG,
      uniforms,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    })
  );
  padMesh.frustumCulled = false;
  scene.add(padMesh);

  const pos = { x: 0, y: 0 };
  const path = new Float32Array(8);

  const writeInstances = () => {
    net.nodes.forEach((node, i) => {
      const l = net.layers[node.layer];
      offset(node, pos);
      aPos.array[i * 3] = pos.x;
      aPos.array[i * 3 + 1] = pos.y;
      aPos.array[i * 3 + 2] = (node.hub ? 0.062 : 0.03) * (0.7 + l.dim * 0.5);
      aPadMeta.array[i * 2] = l.dim;
      aPadMeta.array[i * 2 + 1] = node.hub ? 1 : 0;
    });
    padGeo.instanceCount = net.nodes.length;
    aPos.needsUpdate = true;
    aPadMeta.needsUpdate = true;

    let s = 0;
    for (const trace of net.traces) {
      tracePath(trace, path);
      const l = net.layers[trace.layer];
      for (let k = 0; k < 3 && s < SEGS; k++, s++) {
        aSeg.array[s * 4] = path[k * 2];
        aSeg.array[s * 4 + 1] = path[k * 2 + 1];
        aSeg.array[s * 4 + 2] = path[k * 2 + 2];
        aSeg.array[s * 4 + 3] = path[k * 2 + 3];
        aSegMeta.array[s * 2] = l.width;
        aSegMeta.array[s * 2 + 1] = l.dim;
      }
    }
    traceGeo.instanceCount = s;
    aSeg.needsUpdate = true;
    aSegMeta.needsUpdate = true;
  };

  const syncTheme = () => {
    uniforms.uInk.value = shared.dark ? 0 : 1;
    uniforms.uCeil.value = parseFloat(readVar("--feed-ceiling")) || 0.08;
    uniforms.uStrength.value = parseFloat(readVar("--three-opacity")) || 0.72;
    uniforms.uSurface.value.set(readVar("--md-surface") || "#080f22");
    uniforms.uNode.value.set(readVar("--feed-near") || "#72a2ff");
    if (!shared.accent) uniforms.uLine.value.set(readVar("--feed-line") || "#256eff");
  };
  syncTheme();

  const resize = () => {
    renderer.setPixelRatio(DPR() * config.threeScale);
    renderer.setSize(window.innerWidth, window.innerHeight, false);
    uniforms.uAspectRatio.value = net.aspect;
  };
  resize();

  return {
    canvas,
    resize,
    theme: syncTheme,
    accent(hex) {
      uniforms.uLine.value.set(hex);
    },
    render() {
      uniforms.uAspectRatio.value = net.aspect;
      writeInstances();
      renderer.render(scene, camera);
    },
  };
}

/* ============================================================================
   THE TRAFFIC — pixi.js
   ========================================================================== */

async function buildTraffic(config, shared) {
  const PIXI = await import("../vendor/pixi.min.js");
  const canvas = document.getElementById("bg-pixi");

  const renderer = new PIXI.WebGLRenderer();
  await renderer.init({
    canvas,
    width: window.innerWidth,
    height: window.innerHeight,
    backgroundAlpha: 0,
    antialias: false,
    resolution: Math.min(DPR(), config.pixiRes),
    autoDensity: true,
    powerPreference: "high-performance",
  });

  const stage = new PIXI.Container();
  const dynamic = { position: true, vertex: true, color: true, rotation: true };

  const packetTexture = PIXI.Texture.from(packetCanvas());
  const glyphSheet = PIXI.Texture.from(glyphCanvas());
  const glyphFrames = [];
  for (let i = 0; i < 16; i++) {
    glyphFrames.push(
      new PIXI.Texture({
        source: glyphSheet.source,
        frame: new PIXI.Rectangle(i * 10, 0, 10, 14),
      })
    );
  }

  const flow = new PIXI.ParticleContainer({ dynamicProperties: dynamic });
  const bytes = new PIXI.ParticleContainer({ dynamicProperties: dynamic });
  stage.addChild(flow, bytes);

  let W = window.innerWidth;
  let H = window.innerHeight;

  const packetTint = () => hexToInt(shared.accent || readVar("--feed-packet") || "#72a2ff");
  let tint = packetTint();
  let alphaMax = parseFloat(readVar("--packet-alpha")) || 0.28;

  const make = (container, texture, count, list, speedLo, speedHi) => {
    for (let i = 0; i < count; i++) {
      const particle = new PIXI.Particle({
        texture,
        x: 0,
        y: 0,
        tint,
        alpha: 0,
        anchorX: 0.5,
        anchorY: 0.5,
      });
      container.addParticle(particle);
      list.push({
        p: particle,
        trace: 0,
        d: Math.random(),
        speed: speedLo + Math.random() * (speedHi - speedLo),
        size: 0.7 + Math.random() * 0.6,
        hot: 0,
      });
    }
  };

  const packets = [];
  const glyphs = [];
  make(flow, packetTexture, config.packets, packets, 0.22, 0.46);
  for (let i = 0; i < config.bytes; i++) {
    const particle = new PIXI.Particle({
      texture: glyphFrames[i % 16],
      x: 0,
      y: 0,
      tint,
      alpha: 0,
      anchorX: 0.5,
      anchorY: 0.5,
    });
    bytes.addParticle(particle);
    glyphs.push({
      p: particle,
      trace: 0,
      d: Math.random(),
      speed: 0.12 + Math.random() * 0.14,
      size: 1,
      hot: 0,
    });
  }

  const assign = () => {
    const n = Math.max(1, net.traces.length);
    packets.forEach((p, i) => (p.trace = i % n));
    // Byte streams only run the busier layers, so they read as bulk transfer
    // rather than as noise everywhere.
    glyphs.forEach((g, i) => (g.trace = (i * 3 + 1) % n));
  };
  assign();

  const applyBlend = (dark) => {
    const mode = dark ? "add" : "normal";
    flow.blendMode = mode;
    bytes.blendMode = mode;
  };
  applyBlend(shared.dark);

  const resize = () => {
    W = window.innerWidth;
    H = window.innerHeight;
    renderer.resize(W, H);
    assign();
  };

  const path = new Float32Array(8);
  const world = { x: 0, y: 0, dx: 1, dy: 0 };
  const screen = { x: 0, y: 0 };
  const STEP = 1 / 60;
  let accumulator = 0;

  const advance = (list, scale, glyph) => {
    for (const item of list) {
      const trace = net.traces[item.trace];
      if (!trace) continue;

      tracePath(trace, path);
      const len = pathLength(path);
      if (len <= 0) continue;

      // Constant speed along the run. Linear, because this is data in transit.
      item.d += (item.speed * STEP * (1 + item.hot * 1.5)) / len;
      if (item.hot > 0) item.hot = Math.max(0, item.hot - STEP * 1.3);
      if (item.d >= 1) {
        item.d -= 1;
        // Continue onto whatever the pad it arrived at feeds next.
        const next = net.traces.findIndex((t) => t.from === trace.to);
        if (next >= 0) item.trace = next;
      }

      pathPoint(path, item.d * len, world);
      toScreen(world.x, world.y, W, H, screen);

      const sprite = item.p;
      sprite.x = screen.x;
      sprite.y = screen.y;
      const s = scale * item.size;
      sprite.scaleX = s;
      sprite.scaleY = s;
      // Packets lie along the trace they are on; glyphs stay upright so they
      // stay readable as characters.
      sprite.rotation = glyph ? 0 : Math.atan2(world.dy, world.dx);
      const dim = net.layers[trace.layer].dim;
      sprite.alpha = alphaMax * dim * (glyph ? 0.75 : 1) * (1 + item.hot * 1.4);
      sprite.tint = item.hot > 0.02 ? 0xffffff : tint;
    }
  };

  const stepLogic = () => {
    if (!net.traces.length) return;
    const base = Math.min(W, H) * 0.0011;
    advance(packets, base, false);
    advance(glyphs, base * 0.85, true);
  };

  stepLogic();

  window.addEventListener("net:burst", () => {
    let n = 0;
    for (const packet of packets) {
      if (n++ % 4 === 0) packet.hot = 1;
      if (n > 400) break;
    }
  });

  return {
    canvas,
    resize,
    theme(dark) {
      tint = packetTint();
      alphaMax = parseFloat(readVar("--packet-alpha")) || 0.28;
      applyBlend(dark);
    },
    accent() {
      tint = packetTint();
    },
    render(dt) {
      accumulator += dt;
      let guard = 0;
      while (accumulator >= STEP && guard++ < 4) {
        accumulator -= STEP;
        stepLogic();
      }
      renderer.render(stage);
    },
  };
}

/** A short dash. Packets lie along their trace, so they read as data moving in
 *  a direction rather than as dots sitting on a line. */
function packetCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 16;
  canvas.height = 6;
  const ctx = canvas.getContext("2d");
  const g = ctx.createLinearGradient(0, 0, 16, 0);
  g.addColorStop(0, "rgba(255,255,255,0)");
  g.addColorStop(0.45, "rgba(255,255,255,1)");
  g.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = g;
  ctx.fillRect(0, 1, 16, 4);
  return canvas;
}

/** A strip of the sixteen hex digits, drawn once. Each byte glyph picks a
 *  frame from it, and because they all share one source pixi still batches the
 *  whole stream into a single draw. */
function glyphCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 160;
  canvas.height = 14;
  const ctx = canvas.getContext("2d");
  ctx.font = "700 11px ui-monospace, monospace";
  ctx.fillStyle = "#ffffff";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  "0123456789ABCDEF".split("").forEach((ch, i) => {
    ctx.fillText(ch, i * 10 + 5, 7);
  });
  return canvas;
}
