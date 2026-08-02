/* ============================================================================
   The living background: a triangulated dataflow network.
   ============================================================================

   Nodes on a jittered triangular lattice, edges between neighbours, and
   glowing packets travelling those edges — each with a dimmer dot trailing it
   for a motion streak. Three layers parallax at different rates.

   The division of labour:

   - three.js owns THE NETWORK. Two instanced draw calls: one for the edges,
     one for the nodes. Both are quads placed from the shared model, so the
     geometry is exact.
   - pixi.js owns THE TRAFFIC. Packets and their trails — individually
     positioned, scaled and tinted sprites, which is what its batcher is for.

   Everything is laid out in CSS pixels by mesh.js, so the line weight, node
   size and packet speed are the numbers they claim to be at any DPR. Both
   renderers convert through the same `toClip()`, so a packet is always exactly
   on the edge three.js drew.

   Motion is Material 3 Expressive: springs for anything physical, strictly
   linear for anything representing data in transit.
   ========================================================================== */

import { frameState, reducedMotion } from "./ui.js";
import { SPRING, Spring } from "./spring.js";
import { mesh, layout, nodeAt, toClip } from "./mesh.js";

const DPR = () => Math.min(window.devicePixelRatio || 1, 2);

function hasWebGL() {
  try {
    const canvas = document.createElement("canvas");
    return Boolean(canvas.getContext("webgl2") || canvas.getContext("webgl"));
  } catch (e) {
    return false;
  }
}

/** Tier 1 drops the network and keeps the traffic: the mesh is the expensive
 *  half, and the moving packets are what carry the idea. */
function pickTier() {
  const w = window.innerWidth;
  let tier = w >= 1280 ? 3 : w >= 768 ? 2 : 1;
  if ((navigator.hardwareConcurrency || 8) <= 4) tier = Math.max(1, tier - 1);
  if ((navigator.deviceMemory || 8) <= 4) tier = Math.max(1, tier - 1);
  return tier;
}

const TIERS = {
  3: { network: true, nodes: 900, packets: 220, threeScale: 1, pixiRes: 2 },
  2: { network: true, nodes: 560, packets: 140, threeScale: 0.85, pixiRes: 1.5 },
  1: { network: false, nodes: 260, packets: 70, threeScale: 0, pixiRes: 1 },
};

const readVar = (name) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim();

const hexToInt = (hex) => Number(`0x${(hex || "#ffffff").replace("#", "")}`);

/* ---- Entry --------------------------------------------------------------- */

export async function initBackground() {
  if (!hasWebGL()) return; // The CSS fallback composition is already showing.

  const config = TIERS[pickTier()];
  layout(window.innerWidth, window.innerHeight, config.nodes);

  const shared = {
    dark: document.documentElement.dataset.theme === "dark",
    burst: new Spring(0, SPRING.spatialFast),
    accent: null,
    config,
  };

  const layers = [];

  try {
    if (config.network) layers.push(await buildNetwork(config, shared));
  } catch (error) {
    console.warn("[scenedeck] network layer unavailable:", error);
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
    observeResize(() => {
      layout(window.innerWidth, window.innerHeight, config.nodes);
      layers.forEach((layer) => layer.resize());
      paint();
    });
    return;
  }

  drive(layers, shared);
}

/** ResizeObserver on the document element rather than a window listener: it
 *  also catches the mobile URL bar collapsing, which changes the viewport
 *  height without firing resize on every browser. */
function observeResize(fn) {
  const run = debounce(fn, 140);
  if ("ResizeObserver" in window) new ResizeObserver(run).observe(document.documentElement);
  else window.addEventListener("resize", run);
}

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

    mesh.time += dt;
    mesh.scroll = frameState.progress;

    // The pointer is damped rather than sprung: it is a follow, not an event,
    // and a cursor that overshoots reads as lag.
    const k = 1 - Math.pow(1 - 0.06, dt * 60);
    mesh.pointerX += (frameState.pointerX - mesh.pointerX) * k;
    mesh.pointerY += (frameState.pointerY - mesh.pointerY) * k;

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

  observeResize(() => {
    layout(window.innerWidth, window.innerHeight, shared.config.nodes);
    for (const layer of layers) layer.resize();
  });

  // A program take pushes a burst of traffic across the network.
  window.addEventListener("scenedeck:take", () => {
    shared.burst.velocity += 9;
    shared.burst.to(1);
    clearTimeout(shared.release);
    shared.release = setTimeout(() => shared.burst.to(0), 380);
    window.dispatchEvent(new CustomEvent("mesh:burst"));
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
   THE NETWORK — three.js
   ========================================================================== */

const PRESENT = /* glsl */ `
  uniform float uCeil;
  uniform float uStrength;
  uniform float uInk;
  uniform vec3  uSurface;

  vec3 present(vec3 add) {
    float y = max(dot(add, vec3(0.2126, 0.7152, 0.0722)), 1e-4);
    add *= min(1.0, uCeil / y) * uStrength;

    float density = clamp(dot(add, vec3(0.2126, 0.7152, 0.0722)) / max(uCeil, 1e-4), 0.0, 1.0);
    vec3 outc = max(mix(uSurface + add, mix(uSurface, uSurface * 0.42, density * 0.4), uInk), 0.0);

    // three.js only injects its output encoding into its own materials, so a
    // ShaderMaterial has to encode to sRGB itself.
    return mix(outc * 12.92,
               1.055 * pow(outc, vec3(0.41666)) - 0.055,
               step(vec3(0.0031308), outc));
  }
`;

const EDGE_VERT = /* glsl */ `
  attribute vec4 aSeg;    // clip-space x0, y0, x1, y1
  attribute vec2 aMeta;   // half-thickness in clip Y, dim
  varying vec2 vUv;
  varying float vDim;
  uniform float uAspect;

  void main() {
    vUv = uv;
    vDim = aMeta.y;

    vec2 a = aSeg.xy;
    vec2 b = aSeg.zw;
    // Normalise in pixel proportions, then convert back, so a diagonal edge is
    // the same visual weight as a horizontal one.
    vec2 d = vec2((b.x - a.x) * uAspect, b.y - a.y);
    float len = max(length(d), 1e-6);
    vec2 n = vec2(-d.y, d.x) / len;

    vec2 p = mix(a, b, uv.x) + vec2(n.x / uAspect, n.y) * (uv.y - 0.5) * aMeta.x;
    gl_Position = vec4(p, 0.0, 1.0);
  }
`;

const EDGE_FRAG = /* glsl */ `
  precision highp float;
  varying vec2 vUv;
  varying float vDim;
  uniform vec3 uLine;

  void main() {
    // Soft across the width so a 1.5px line has an edge rather than a stair.
    float cov = 1.0 - smoothstep(0.2, 0.5, abs(vUv.y - 0.5));
    gl_FragColor = vec4(present(uLine * vDim) * cov, cov);
  }
`;

const NODE_VERT = /* glsl */ `
  attribute vec3 aPos;    // clip x, clip y, half-size in clip Y
  attribute vec2 aMeta;   // dim, hub
  varying vec2 vUv;
  varying float vDim;
  varying float vHub;
  uniform float uAspect;

  void main() {
    vUv = uv;
    vDim = aMeta.x;
    vHub = aMeta.y;
    vec2 p = aPos.xy + vec2(position.x / uAspect, position.y) * aPos.z * 2.0;
    gl_Position = vec4(p, 0.0, 1.0);
  }
`;

const NODE_FRAG = /* glsl */ `
  precision highp float;
  varying vec2 vUv;
  varying float vDim;
  varying float vHub;
  uniform vec3 uLine;
  uniform vec3 uNode;

  void main() {
    float r = length(vUv - 0.5) * 2.0;
    float cov = 1.0 - smoothstep(0.55, 1.0, r);
    if (cov < 0.004) discard;

    // Junctions read brighter than the lattice points they sit among.
    vec3 tint = mix(uNode, uLine, 0.3);
    gl_FragColor = vec4(present(tint * vDim * (1.0 + vHub * 1.4)) * cov, cov);
  }
`;

async function buildNetwork(config, shared) {
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
    uAspect: { value: 1 },
    uCeil: { value: 0.08 },
    uStrength: { value: 0.72 },
    uInk: { value: shared.dark ? 0 : 1 },
    uSurface: { value: new THREE.Color("#080f22") },
    uLine: { value: new THREE.Color("#8ab4f8") },
    uNode: { value: new THREE.Color("#c2d7ff") },
  };

  const quad = new THREE.PlaneGeometry(1, 1);
  const clone = () => {
    const g = new THREE.InstancedBufferGeometry();
    g.index = quad.index;
    g.setAttribute("position", quad.attributes.position);
    g.setAttribute("uv", quad.attributes.uv);
    return g;
  };

  const MAX_EDGES = config.nodes * 3;
  const edgeGeo = clone();
  const aSeg = new THREE.InstancedBufferAttribute(new Float32Array(MAX_EDGES * 4), 4);
  const aEdgeMeta = new THREE.InstancedBufferAttribute(new Float32Array(MAX_EDGES * 2), 2);
  edgeGeo.setAttribute("aSeg", aSeg);
  edgeGeo.setAttribute("aMeta", aEdgeMeta);

  const edgeMesh = new THREE.Mesh(
    edgeGeo,
    new THREE.ShaderMaterial({
      vertexShader: PRESENT + EDGE_VERT,
      fragmentShader: PRESENT + EDGE_FRAG,
      uniforms,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    })
  );
  edgeMesh.frustumCulled = false;
  scene.add(edgeMesh);

  const nodeGeo = clone();
  const aPos = new THREE.InstancedBufferAttribute(new Float32Array(config.nodes * 3), 3);
  const aNodeMeta = new THREE.InstancedBufferAttribute(new Float32Array(config.nodes * 2), 2);
  nodeGeo.setAttribute("aPos", aPos);
  nodeGeo.setAttribute("aMeta", aNodeMeta);

  const nodeMesh = new THREE.Mesh(
    nodeGeo,
    new THREE.ShaderMaterial({
      vertexShader: PRESENT + NODE_VERT,
      fragmentShader: PRESENT + NODE_FRAG,
      uniforms,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    })
  );
  nodeMesh.frustumCulled = false;
  scene.add(nodeMesh);

  const pa = { x: 0, y: 0 };
  const pb = { x: 0, y: 0 };
  const ca = { x: 0, y: 0 };
  const cb = { x: 0, y: 0 };

  // 1.5px lines and 2.2px dots, expressed as clip-space Y so the shader can
  // correct for aspect and keep them circular.
  const EDGE_PX = 1.5;
  const NODE_PX = 2.2;

  const writeInstances = () => {
    const toClipY = 2 / Math.max(1, mesh.height);

    mesh.nodes.forEach((node, i) => {
      nodeAt(node, pa);
      toClip(pa.x, pa.y, ca);
      aPos.array[i * 3] = ca.x;
      aPos.array[i * 3 + 1] = ca.y;
      aPos.array[i * 3 + 2] = (NODE_PX * (node.hub ? 1.7 : 1) * 0.5) * toClipY;
      aNodeMeta.array[i * 2] = mesh.layers[node.layer].dim;
      aNodeMeta.array[i * 2 + 1] = node.hub ? 1 : 0;
    });
    nodeGeo.instanceCount = mesh.nodes.length;
    aPos.needsUpdate = true;
    aNodeMeta.needsUpdate = true;

    let e = 0;
    for (const edge of mesh.edges) {
      if (e >= MAX_EDGES) break;
      nodeAt(mesh.nodes[edge.a], pa);
      nodeAt(mesh.nodes[edge.b], pb);
      toClip(pa.x, pa.y, ca);
      toClip(pb.x, pb.y, cb);
      aSeg.array[e * 4] = ca.x;
      aSeg.array[e * 4 + 1] = ca.y;
      aSeg.array[e * 4 + 2] = cb.x;
      aSeg.array[e * 4 + 3] = cb.y;
      aEdgeMeta.array[e * 2] = EDGE_PX * toClipY;
      aEdgeMeta.array[e * 2 + 1] = mesh.layers[edge.layer].dim;
      e++;
    }
    edgeGeo.instanceCount = e;
    aSeg.needsUpdate = true;
    aEdgeMeta.needsUpdate = true;
  };

  const syncTheme = () => {
    uniforms.uInk.value = shared.dark ? 0 : 1;
    uniforms.uCeil.value = parseFloat(readVar("--feed-ceiling")) || 0.08;
    uniforms.uStrength.value = parseFloat(readVar("--three-opacity")) || 0.72;
    uniforms.uSurface.value.set(readVar("--md-surface") || "#080f22");
    uniforms.uNode.value.set(readVar("--mesh-node") || "#c2d7ff");
    if (!shared.accent) uniforms.uLine.value.set(readVar("--mesh-edge") || "#8ab4f8");
  };
  syncTheme();

  const resize = () => {
    renderer.setPixelRatio(DPR() * config.threeScale);
    renderer.setSize(window.innerWidth, window.innerHeight, false);
    uniforms.uAspect.value = mesh.width / Math.max(1, mesh.height);
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
      uniforms.uAspect.value = mesh.width / Math.max(1, mesh.height);
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
  const texture = PIXI.Texture.from(dotCanvas());

  const flow = new PIXI.ParticleContainer({
    dynamicProperties: { position: true, vertex: true, color: true, rotation: false },
  });
  stage.addChild(flow);

  // Google blue, a lighter blue, and a healthy green — the packet colours
  // cycle so the network reads as carrying more than one kind of traffic.
  const palette = () => [
    hexToInt(readVar("--mesh-edge") || "#8ab4f8"),
    hexToInt(readVar("--mesh-node") || "#c2d7ff"),
    hexToInt(readVar("--inst-ok") || "#57e19c"),
  ];
  let colours = palette();
  let alphaMax = parseFloat(readVar("--packet-alpha")) || 0.28;

  const packets = [];
  for (let i = 0; i < config.packets; i++) {
    const head = new PIXI.Particle({
      texture,
      x: -50,
      y: -50,
      tint: colours[i % 3],
      alpha: 0,
      anchorX: 0.5,
      anchorY: 0.5,
    });
    const trail = new PIXI.Particle({
      texture,
      x: -50,
      y: -50,
      tint: colours[i % 3],
      alpha: 0,
      anchorX: 0.5,
      anchorY: 0.5,
    });
    flow.addParticle(head);
    flow.addParticle(trail);
    packets.push({
      head,
      trail,
      edge: 0,
      d: Math.random(),
      // 42-120 px/s.
      speed: 42 + Math.random() * 78,
      colour: i % 3,
      hot: 0,
    });
  }

  const assign = () => {
    const n = Math.max(1, mesh.edges.length);
    packets.forEach((p, i) => {
      p.edge = (i * 7 + 3) % n;
      p.d = Math.random();
    });
  };
  assign();

  const applyBlend = (dark) => {
    flow.blendMode = dark ? "add" : "normal";
  };
  applyBlend(shared.dark);

  const resize = () => {
    renderer.resize(window.innerWidth, window.innerHeight);
    assign();
  };

  const pa = { x: 0, y: 0 };
  const pb = { x: 0, y: 0 };
  const STEP = 1 / 60;
  const TRAIL_PX = 14;
  let accumulator = 0;

  const place = (sprite, x, y, size, alpha, tintValue) => {
    sprite.x = x;
    sprite.y = y;
    sprite.scaleX = size;
    sprite.scaleY = size;
    sprite.alpha = alpha;
    sprite.tint = tintValue;
  };

  const stepLogic = () => {
    if (!mesh.edges.length) return;

    for (const packet of packets) {
      const edge = mesh.edges[packet.edge];
      if (!edge) continue;

      nodeAt(mesh.nodes[edge.a], pa);
      nodeAt(mesh.nodes[edge.b], pb);
      const dx = pb.x - pa.x;
      const dy = pb.y - pa.y;
      const len = Math.hypot(dx, dy);
      if (len <= 0) continue;

      // Constant pixels per second. Linear — this is data in transit.
      packet.d += (packet.speed * STEP * (1 + packet.hot * 1.4)) / len;
      if (packet.hot > 0) packet.hot = Math.max(0, packet.hot - STEP * 1.2);

      if (packet.d >= 1) {
        packet.d -= 1;
        // Continue along an edge leaving the node it just arrived at, so a
        // packet crosses the network rather than looping one segment.
        let next = -1;
        for (let k = 1; k <= 6; k++) {
          const candidate = mesh.edges[(packet.edge + k * 5) % mesh.edges.length];
          if (candidate && candidate.a === edge.b) {
            next = (packet.edge + k * 5) % mesh.edges.length;
            break;
          }
        }
        packet.edge = next >= 0 ? next : Math.floor(Math.random() * mesh.edges.length);
      }

      const dim = mesh.layers[edge.layer].dim;
      const tintValue = packet.hot > 0.02 ? 0xffffff : colours[packet.colour];
      const x = pa.x + dx * packet.d;
      const y = pa.y + dy * packet.d;

      place(packet.head, x, y, 1, alphaMax * dim * (1 + packet.hot * 1.5), tintValue);

      // The trail sits a fixed 14px behind along the same edge, dimmer — a
      // motion streak without a second pass or a filter.
      const t = TRAIL_PX / len;
      const td = packet.d - t;
      place(
        packet.trail,
        td >= 0 ? x - dx * t : x - (dx / len) * TRAIL_PX,
        td >= 0 ? y - dy * t : y - (dy / len) * TRAIL_PX,
        0.72,
        alphaMax * dim * 0.4,
        tintValue
      );
    }
  };

  stepLogic();

  window.addEventListener("mesh:burst", () => {
    let n = 0;
    for (const packet of packets) {
      if (n++ % 3 === 0) packet.hot = 1;
    }
  });

  return {
    canvas,
    resize,
    theme(dark) {
      colours = palette();
      alphaMax = parseFloat(readVar("--packet-alpha")) || 0.28;
      applyBlend(dark);
    },
    accent(hex) {
      colours = [hexToInt(hex), colours[1], colours[2]];
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

/** A small glowing dot. The falloff is baked in, which is what lets additive
 *  blending look like a glow with no filter pass and no render target. */
function dotCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 16;
  canvas.height = 16;
  const ctx = canvas.getContext("2d");
  const glow = ctx.createRadialGradient(8, 8, 0, 8, 8, 8);
  glow.addColorStop(0, "rgba(255,255,255,1)");
  glow.addColorStop(0.28, "rgba(255,255,255,0.85)");
  glow.addColorStop(0.6, "rgba(255,255,255,0.18)");
  glow.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, 16, 16);
  return canvas;
}
