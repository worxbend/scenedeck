/* ============================================================================
   The living background: the OBS WebSocket link, seen from inside it.
   ============================================================================

   SceneDeck runs on one machine and OBS runs on another with a WebSocket
   between them, so the background is that link: a channel receding to a
   vanishing point, eight bit-lanes along its wall, encoded video travelling up
   it toward the camera and a thinner counter-flow of commands going the other
   way. Every two seconds — OBS's default keyframe interval — a GOP boundary
   sweeps down the channel and the stream resets.

   The division of labour is real:

   - three.js owns THE CHANNEL. One full-screen shader with a closed-form
     infinite-cylinder intersection: because the eye sits on the channel axis
     the intersection solves analytically, so there is no ray marching and the
     cost is constant per pixel. Two triangles, one draw call, no textures, no
     render targets, no depth buffer.
   - pixi.js owns THE PACKETS. Thousands of individually positioned, scaled and
     tinted sprites, which is exactly what its batcher exists for. In three.js
     this would mean hand-rolling a points pipeline with custom attributes for
     perspective-correct size and per-particle tint, for more code and no gain.

   What makes them one image is wire.js: both layers use the same projection
   and the same centreline, so a packet on lane 3 at z = 12 sits exactly on the
   shader's rail for lane 3 at z = 12, at any aspect ratio.

   Motion is Material 3 Expressive — springs, not curves. Anything physical
   moves on a spatial spring and is allowed to overshoot. Telemetry (the raster
   scroll, packet velocity, the GOP clock, a dropped frame fading out) is
   strictly linear, because easing a meter is a lie.
   ========================================================================== */

import { frameState, reducedMotion } from "./ui.js";
import { SPRING, Spring } from "./spring.js";
import { wire, project } from "./wire.js";

const DPR = () => Math.min(window.devicePixelRatio || 1, 2);

function hasWebGL() {
  try {
    const canvas = document.createElement("canvas");
    return Boolean(canvas.getContext("webgl2") || canvas.getContext("webgl"));
  } catch (e) {
    return false;
  }
}

/** Tier 1 drops the channel shader and keeps the packets: the shader is the
 *  expensive half, and the packets are what carries the idea. */
function pickTier() {
  const w = window.innerWidth;
  let tier = w >= 1280 ? 3 : w >= 768 ? 2 : 1;
  if ((navigator.hardwareConcurrency || 8) <= 4) tier = Math.max(1, tier - 1);
  if ((navigator.deviceMemory || 8) <= 4) tier = Math.max(1, tier - 1);
  return tier;
}

const TIERS = {
  3: { channel: true, packets: 3200, drops: 240, threeScale: 0.72, pixiRes: 1.5 },
  2: { channel: true, packets: 1800, drops: 160, threeScale: 0.62, pixiRes: 1.25 },
  1: { channel: false, packets: 700, drops: 80, threeScale: 0, pixiRes: 1 },
};

const readVar = (name) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim();

const hexToInt = (hex) => Number(`0x${(hex || "#ffffff").replace("#", "")}`);

function smoothstep(a, b, x) {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
}

/* ---- Entry --------------------------------------------------------------- */

export async function initBackground() {
  if (!hasWebGL()) return; // The CSS fallback composition is already showing.

  const config = TIERS[pickTier()];

  const shared = {
    dark: document.documentElement.dataset.theme === "dark",
    // The keyframe ring's depth, on the one Expressive token with real
    // overshoot — so the ring visibly punches past the eye before settling.
    keyZ: new Spring(wire.Z_FAR, SPRING.spatialFast),
    take: new Spring(0, SPRING.spatialFast),
    vpX: new Spring(0.3, SPRING.spatialSlow),
    vpY: new Spring(-0.02, SPRING.spatialSlow),
    accent: null,
    gopClock: 0,
  };

  const layers = [];

  try {
    if (config.channel) layers.push(await buildChannel(config, shared));
  } catch (error) {
    console.warn("[scenedeck] channel layer unavailable:", error);
  }

  try {
    layers.push(await buildPackets(config, shared));
  } catch (error) {
    console.warn("[scenedeck] packet layer unavailable:", error);
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
/* One requestAnimationFrame for the page. It cancels outright when the tab is
   hidden or the window loses focus, so both renderers cost nothing at all
   rather than idling. */

function drive(layers, shared) {
  let raf = 0;
  let last = 0;
  let running = false;

  const frame = (now) => {
    raf = requestAnimationFrame(frame);
    const dt = Math.min((now - last) / 1000, 0.0333);
    last = now;

    wire.time += dt;
    wire.scroll = frameState.progress;

    // The pointer is damped rather than sprung: it is a follow, not an event,
    // and a cursor that overshoots reads as lag.
    const k = 1 - Math.pow(1 - 0.06, dt * 60);
    wire.pointerX += (frameState.pointerX - wire.pointerX) * k;
    wire.pointerY += (frameState.pointerY - wire.pointerY) * k;

    // The vanishing point leans with the pointer and rises as you scroll, so
    // the channel appears to bank. Slow spatial spring.
    shared.vpX.to(0.3 + wire.pointerX * 0.07);
    shared.vpY.to(-0.02 - wire.pointerY * 0.05 - wire.scroll * 0.18);
    wire.vpX = shared.vpX.step(dt);
    wire.vpY = shared.vpY.step(dt);

    // The GOP clock is a wall clock, not an animation, so it is linear.
    shared.gopClock += dt * 1000;
    if (shared.gopClock >= wire.GOP_MS) {
      shared.gopClock -= wire.GOP_MS;
      launchKeyframe(shared);
    }

    wire.keyZ = shared.keyZ.step(dt);
    wire.takeEnergy = shared.take.step(dt);

    // Bitrate and congestion wander the way a real encoder does: mostly
    // steady, with occasional pressure. Linear — they are telemetry.
    wire.bitrate = 0.62 + 0.2 * Math.sin(wire.time * 0.23) + 0.08 * Math.sin(wire.time * 0.77);
    wire.congestion = Math.max(
      0,
      0.1 + 0.12 * Math.sin(wire.time * 0.11) + 0.08 * Math.sin(wire.time * 0.41 + 2.1)
    );

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
      for (const layer of layers) layer.resize();
    }, 120)
  );

  // A program take forces an I-frame, which is literally true of an OBS scene
  // switch — so the take and the keyframe are the same gesture.
  let takeRelease = 0;
  window.addEventListener("scenedeck:take", () => {
    shared.gopClock = 0;
    launchKeyframe(shared);
    shared.take.velocity += 9;
    shared.take.to(1);
    clearTimeout(takeRelease);
    takeRelease = setTimeout(() => shared.take.to(0), 420);
    window.dispatchEvent(new CustomEvent("wire:flash"));
  });

  start();
  layers.forEach((layer) => layer.canvas.setAttribute("data-live", ""));
}

/** Relaunch the GOP ring from the far end of the channel. */
function launchKeyframe(shared) {
  shared.keyZ.set(wire.Z_FAR);
  shared.keyZ.velocity = -34;
  shared.keyZ.to(-2);
}

function debounce(fn, ms) {
  let timer = 0;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

/* ============================================================================
   THE CHANNEL — three.js
   ========================================================================== */

const VERT = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`;

const FRAG = /* glsl */ `
  precision highp float;
  varying vec2 vUv;

  uniform float uTime;
  uniform float uAspect;
  uniform vec2  uVp;
  uniform vec2  uPointer;
  uniform float uBitrate;
  uniform float uCongestion;
  uniform float uKeyZ;
  uniform float uTake;
  uniform float uInk;       // 0 dark, 1 light
  uniform float uCeil;      // hard luminance ceiling
  uniform vec3  uNear;      // #065dac, the near end of the OBS hero gradient
  uniform vec3  uFar;       // #280f83, the far end
  uniform vec3  uRail;      // lane rails, overridable by a scene accent
  uniform vec3  uSurface;   // page surface — composited here, not by CSS
  uniform float uStrength;  // how strongly the channel reads over the surface

  const float TAU    = 6.28318530718;
  const float FOCAL  = 1.7320508075688772;
  const float RADIUS = 1.35;

  // MUST match centreline() in wire.js exactly, or the layers separate.
  vec2 centreline(float z, float t, vec2 ptr) {
    float k = z * 0.0125;
    return vec2(0.62 * sin(z * 0.0600 + t * 0.150) + ptr.x * k,
                0.44 * cos(z * 0.0450 + t * 0.110) + ptr.y * k);
  }

  float h21(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123); }

  float vnoise(vec2 p) {
    vec2 i = floor(p), f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(h21(i), h21(i + vec2(1.0, 0.0)), f.x),
               mix(h21(i + vec2(0.0, 1.0)), h21(i + vec2(1.0, 1.0)), f.x), f.y);
  }

  void main() {
    vec2 ndc = (vUv * 2.0 - 1.0) - uVp;
    vec3 rd  = normalize(vec3(ndc.x * uAspect / FOCAL, ndc.y / FOCAL, -1.0));

    // The eye sits exactly on the channel axis, so intersecting an infinite
    // cylinder is closed form: no marching, constant cost per pixel.
    float rl = max(length(rd.xy), 1e-4);
    float s  = RADIUS / rl;
    float z  = min(-rd.z * s, 400.0);

    float ang = atan(rd.y, rd.x);
    float u   = ang / TAU + 0.5;
    float v   = z * 0.075 - uTime * 0.42;   // raster scroll: strictly linear

    vec2 c = centreline(z, uTime, uPointer);
    u += (c.x * 0.11 + c.y * 0.07) / max(z, 1.0);

    // Congestion warps depth rather than the surface, so it reads as the link
    // buffering rather than as water rippling.
    v += (vnoise(vec2(u * 6.0, v * 3.0 - uTime * 0.9)) - 0.5) * uCongestion * 0.30;

    // Everything converges at the vanishing point, so that is the one place
    // the structure terms stack into a hot core. Damp by distance from it.
    float vpFall = 0.22 + 0.78 * smoothstep(0.0, 0.62, length(ndc));

    // 1. eight bit-lanes, brightening with bitrate
    float lane  = abs(fract(u * 8.0) - 0.5) * 2.0;
    float rails = pow(1.0 - lane, 34.0) * (0.55 + 0.45 * uBitrate) * vpFall;

    // 2. the raster structure of the video signal itself
    float raster = pow(0.5 + 0.5 * sin(v * 62.0), 3.0);

    // 3. 4:2:0 macroblocks, visible only when the link is congested
    vec2  mb    = floor(vec2(u * 64.0, v * 22.0));
    float macro = step(1.0 - uCongestion * 0.55, vnoise(mb + floor(uTime * 6.0) * 7.13))
                  * uCongestion * 0.55;

    // 4. GOP ribs running down the channel
    float rib = pow(1.0 - abs(fract(v * 0.25) * 2.0 - 1.0), 40.0);

    // 5. the keyframe ring, sweeping toward the eye
    float ring = exp(-abs(z - uKeyZ) * (2.4 - uTake * 0.8)) * (0.95 + 1.10 * uTake)
                 * smoothstep(0.9, 5.5, uKeyZ);

    // Depth ramp: violet far, blue near. The OBS hero gradient emitted along
    // the channel rather than painted across it.
    float depth = clamp(z / 40.0, 0.0, 1.0);
    vec3  hue   = mix(uNear, uFar, depth);

    float structure = (raster * 0.55 + rib * 0.45 + macro) * vpFall + rails * 0.70;
    // Fade the far end out so the channel has no visible end wall.
    float fade = 1.0 - smoothstep(26.0, 40.0, z) * 0.85;

    // DARK: the channel is emissive over the page surface. The ceiling is a
    // clamp on what it may add, not a scale — so no accent, take or keyframe
    // can push the background bright enough to hurt body copy.
    vec3 glow = (hue * (0.30 + structure) + uRail * (rails * 0.30 + ring * 0.75)) * fade;
    float y = max(dot(glow, vec3(0.2126, 0.7152, 0.0722)), 1e-4);
    glow *= min(1.0, uCeil / y) * uStrength;
    vec3 lit = uSurface + glow;

    // LIGHT: the same structure printed as ink. It has to darken the paper
    // TOWARD the wire's own hue — subtracting a blue glow from white paper
    // would leave its complement, and the first attempt came out orange.
    float density = clamp(structure * 0.85 + ring * 0.55, 0.0, 1.0) * fade;
    vec3  ink     = mix(uSurface, hue * 0.40, density * uCeil * 2.4 * uStrength);

    // The canvas is opaque: the surface is composited here rather than by
    // letting a translucent canvas reveal the CSS fallback underneath.
    vec3 outc = max(mix(lit, ink, uInk), 0.0);

    // three.js only injects its output-encoding chunk into its own materials,
    // so a ShaderMaterial has to encode to sRGB itself. Without this the same
    // --md-surface hex renders a visibly different tone on the canvas than the
    // CSS around it. Including <colorspace_fragment> does not work here: it
    // calls linearToOutputTexel, which is not declared for ShaderMaterial, and
    // the program fails to link.
    vec3 encoded = mix(outc * 12.92,
                       1.055 * pow(outc, vec3(0.41666)) - 0.055,
                       step(vec3(0.0031308), outc));

    gl_FragColor = vec4(encoded, 1.0);
  }
`;

async function buildChannel(config, shared) {
  const THREE = await import("../vendor/three.min.js");
  const canvas = document.getElementById("bg-three");

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: false,
    alpha: false,
    depth: false,
    stencil: false,
    powerPreference: "high-performance",
  });
  renderer.outputColorSpace = THREE.SRGBColorSpace;

  const scene = new THREE.Scene();
  // The shader builds its own rays from vUv, so the camera never transforms
  // anything — it exists because three.js requires one.
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);

  const uniforms = {
    uTime: { value: 0 },
    uAspect: { value: 1 },
    uVp: { value: new THREE.Vector2(0.3, -0.02) },
    uPointer: { value: new THREE.Vector2(0, 0) },
    uBitrate: { value: 0.62 },
    uCongestion: { value: 0.1 },
    uKeyZ: { value: wire.Z_FAR },
    uTake: { value: 0 },
    uInk: { value: shared.dark ? 0 : 1 },
    uCeil: { value: 0.1 },
    uStrength: { value: 0.72 },
    uNear: { value: new THREE.Color("#065dac") },
    uFar: { value: new THREE.Color("#280f83") },
    uRail: { value: new THREE.Color("#256eff") },
    uSurface: { value: new THREE.Color("#080f22") },
  };

  const mesh = new THREE.Mesh(
    new THREE.PlaneGeometry(2, 2),
    new THREE.ShaderMaterial({
      vertexShader: VERT,
      fragmentShader: FRAG,
      uniforms,
      depthTest: false,
      depthWrite: false,
    })
  );
  mesh.frustumCulled = false;
  scene.add(mesh);

  const syncTheme = () => {
    uniforms.uInk.value = shared.dark ? 0 : 1;
    uniforms.uCeil.value = parseFloat(readVar("--wire-ceiling")) || (shared.dark ? 0.1 : 0.3);
    uniforms.uStrength.value = parseFloat(readVar("--three-opacity")) || 0.72;
    uniforms.uSurface.value.set(readVar("--md-surface") || "#080f22");
    uniforms.uNear.value.set(readVar("--wire-near") || "#065dac");
    uniforms.uFar.value.set(readVar("--wire-far") || "#280f83");
    if (!shared.accent) uniforms.uRail.value.set(readVar("--wire-rail") || "#256eff");
  };
  syncTheme();

  const resize = () => {
    const w = window.innerWidth;
    const h = window.innerHeight;
    renderer.setPixelRatio(DPR() * config.threeScale);
    renderer.setSize(w, h, false);
    uniforms.uAspect.value = w / Math.max(1, h);
    wire.aspect = uniforms.uAspect.value;
  };
  resize();

  return {
    canvas,
    resize,
    theme: syncTheme,
    accent(hex) {
      uniforms.uRail.value.set(hex);
    },
    render() {
      uniforms.uTime.value = wire.time;
      uniforms.uVp.value.set(wire.vpX, wire.vpY);
      uniforms.uPointer.value.set(wire.pointerX, wire.pointerY);
      uniforms.uBitrate.value = wire.bitrate;
      uniforms.uCongestion.value = wire.congestion;
      uniforms.uKeyZ.value = wire.keyZ;
      uniforms.uTake.value = wire.takeEnergy;
      renderer.render(scene, camera);
    },
  };
}

/* ============================================================================
   THE PACKETS — pixi.js
   ========================================================================== */

async function buildPackets(config, shared) {
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

  const packetTexture = PIXI.Texture.from(packetCanvas());
  const dropTexture = PIXI.Texture.from(dropCanvas());

  // Position, size and tint all change every frame; rotation never does.
  const dynamic = { position: true, vertex: true, color: true, rotation: false };
  const flow = new PIXI.ParticleContainer({ dynamicProperties: dynamic });
  const drops = new PIXI.ParticleContainer({ dynamicProperties: dynamic });
  stage.addChild(flow, drops);

  let W = window.innerWidth;
  let H = window.innerHeight;
  let halfW = W / 2;
  let halfH = H / 2;
  wire.aspect = W / Math.max(1, H);

  const proj = { ndcX: 0, ndcY: 0, scale: 1, shift: 0 };

  const packetTint = () => hexToInt(shared.accent || readVar("--wire-packet") || "#72a2ff");
  const dropTint = () => hexToInt(readVar("--wire-drop") || "#e74c3c");

  let tint = packetTint();
  let alphaMax = parseFloat(readVar("--packet-alpha")) || 0.55;
  // On paper a glow sprite is subtractive: the same alpha that reads as bloom
  // on navy prints as a dark patch on white, and the ring was the darkest
  // thing on the light page while it was travelling.

  /* -- the flow ---------------------------------------------------------- */
  const packets = [];
  for (let i = 0; i < config.packets; i++) {
    const particle = new PIXI.Particle({ texture: packetTexture, x: 0, y: 0, tint, alpha: 0, anchorX: 0.5, anchorY: 0.5 });
    flow.addParticle(particle);
    // A fifth of the traffic runs the other way on a tighter radius: video in,
    // commands out. Two machines, one wire, traffic in both directions.
    const outbound = i % 5 === 0;
    packets.push({
      p: particle,
      lane: i % wire.LANES,
      radius: outbound ? 0.6 + Math.random() * 0.14 : 0.86 + Math.random() * 0.135,
      z: wire.Z_NEAR + Math.random() * (wire.Z_FAR - wire.Z_NEAR),
      speed: (5.4 + Math.random() * 3.7) * (outbound ? -1.35 : 1),
      size: 0.55 + Math.random() * 0.7,
      outbound,
      hot: 0,
    });
  }

  /* -- dropped frames ----------------------------------------------------- */
  const dropList = [];
  for (let i = 0; i < config.drops; i++) {
    const particle = new PIXI.Particle({ texture: dropTexture, x: 0, y: 0, tint: dropTint(), alpha: 0, anchorX: 0.5, anchorY: 0.5 });
    drops.addParticle(particle);
    dropList.push({ p: particle, life: 0, x: 0, y: 0, scale: 1 });
  }
  let dropCursor = 0;

  const applyBlend = (dark) => {
    const mode = dark ? "add" : "normal";
    flow.blendMode = mode;
    drops.blendMode = mode;
  };
  applyBlend(shared.dark);

  const resize = () => {
    W = window.innerWidth;
    H = window.innerHeight;
    halfW = W / 2;
    halfH = H / 2;
    wire.aspect = W / Math.max(1, H);
    renderer.resize(W, H);
    // The reduced-motion path paints one frame and never ticks, so without
    // this the packets keep the pixel coordinates of the previous aspect.
    stepLogic();
  };

  const spawnDrop = (x, y, scale) => {
    const drop = dropList[dropCursor];
    dropCursor = (dropCursor + 1) % dropList.length;
    drop.life = 0.42;
    drop.x = x;
    drop.y = y;
    drop.scale = scale;
  };

  const STEP = 1 / 60;
  let accumulator = 0;

  const stepLogic = () => {
    const activeCount = Math.round(packets.length * (0.35 + 0.65 * wire.bitrate));
    const dropChance = wire.congestion * wire.congestion * 0.3;

    for (let i = 0; i < packets.length; i++) {
      const packet = packets[i];
      const sprite = packet.p;

      // Bitrate is literally packet density. Inactive packets collapse to a
      // degenerate quad, so they cost no fill — though pixi still uploads
      // their buffer entries, so they are cheap rather than free.
      if (i >= activeCount) {
        sprite.scaleX = 0;
        sprite.scaleY = 0;
        sprite.alpha = 0;
        continue;
      }

      packet.z -= packet.speed * STEP * (1 + packet.hot * 0.9);
      if (packet.hot > 0) packet.hot = Math.max(0, packet.hot - STEP * 1.1);

      if (packet.z <= wire.Z_NEAR || packet.z >= wire.Z_FAR) {
        // A congested link loses packets: this one never arrives.
        if (!packet.outbound && sprite.alpha > 0.01 && Math.random() < dropChance) {
          spawnDrop(sprite.x, sprite.y, wire.FOCAL / packet.z);
        }
        packet.z = packet.outbound ? wire.Z_NEAR : wire.Z_FAR;
      }

      if (!project(packet.lane, packet.radius, packet.z, proj)) {
        sprite.scaleX = 0;
        sprite.scaleY = 0;
        sprite.alpha = 0;
        continue;
      }

      sprite.x = halfW + proj.ndcX * halfW;
      sprite.y = halfH - proj.ndcY * halfH;
      const scale = proj.scale * packet.size * 2.4;
      sprite.scaleX = scale;
      sprite.scaleY = scale;
      // Packets fade as they reach the eye rather than becoming huge bright
      // squares over the headline — they are consumed on arrival.
      sprite.alpha =
        alphaMax *
        smoothstep(2, 6.5, packet.z) *
        (1 - smoothstep(13, 26, packet.z)) *
        (packet.outbound ? 0.55 : 1) *
        (1 + packet.hot * 1.6);
      sprite.tint = packet.hot > 0.02 ? 0xffffff : tint;
    }

    for (const drop of dropList) {
      if (drop.life <= 0) {
        drop.p.alpha = 0;
        drop.p.scaleX = 0;
        drop.p.scaleY = 0;
        continue;
      }
      drop.life -= STEP;
      const t = 1 - Math.max(0, drop.life) / 0.42;
      drop.p.x = drop.x;
      drop.p.y = drop.y;
      const s = drop.scale * (1 + t * 0.9) * 2.4;
      drop.p.scaleX = s;
      drop.p.scaleY = s;
      // A dropped frame does not bounce. Linear fade, no spring.
      drop.p.alpha = Math.max(0, 0.9 * (1 - t));
    }

  };

  // Seed one pass so the reduced-motion path composes properly instead of
  // drawing every packet stacked at the origin.
  stepLogic();

  // A take flashes the packets nearest the eye white and speeds them up.
  window.addEventListener("wire:flash", () => {
    let flashed = 0;
    for (const packet of packets) {
      if (packet.z < 14 && !packet.outbound && flashed < 220) {
        packet.hot = 1;
        flashed++;
      }
    }
  });

  return {
    canvas,
    resize,
    theme(dark) {
      tint = packetTint();
      alphaMax = parseFloat(readVar("--packet-alpha")) || 0.55;
      for (const drop of dropList) drop.p.tint = dropTint();
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

/* ---- Procedural textures ------------------------------------------------- */
/* Packets are squares because they are data blocks, and because Material 3
   Expressive's shape direction is bolder, not rounder — so they keep a crisp
   corner with only the extra-small radius softening it. */

function packetCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 8;
  canvas.height = 8;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#ffffff";
  if (ctx.roundRect) {
    ctx.beginPath();
    ctx.roundRect(1, 1, 6, 6, 1.5);
    ctx.fill();
  } else {
    ctx.fillRect(1, 1, 6, 6);
  }
  return canvas;
}

/** A broken square: a packet that failed. Reads at 3-9px. */
function dropCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 8;
  canvas.height = 8;
  const ctx = canvas.getContext("2d");
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.moveTo(1.5, 1.5);
  ctx.lineTo(6.5, 6.5);
  ctx.moveTo(6.5, 1.5);
  ctx.lineTo(1.5, 6.5);
  ctx.stroke();
  return canvas;
}

