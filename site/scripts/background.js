/* ============================================================================
   The living background: a rack of anodized panels with signal running across
   them.
   ============================================================================

   Two renderers, one machine. The division of labour is real rather than
   decorative:

   - three.js owns THE METAL. Lit geometry, a genuine studio reflection off a
     procedural environment map, and real z-separation across three depth
     bands. One instanced draw call for every panel on screen.
   - pixi.js owns THE SIGNAL. Thousands of unlit, individually tinted, batched
     2D sprites — telemetry flowing along the panel seams, VU meters, tally
     lamps. That is precisely what pixi's batcher is best at, and doing it in
     three.js would mean hand-rolling a points pipeline for no gain.

   What makes them read as one image rather than two canvases is `rackLines`:
   seven normalised screen-space Y positions computed once per resize and
   consumed by BOTH renderers, so the signal travels exactly along the seams
   between panel rows. Both also share one scroll value, one pointer value and
   one rAF.

   Everything is gated: no WebGL means neither module is even fetched.
   ========================================================================== */

import { frameState, reducedMotion } from "./ui.js";

const DPR = () => Math.min(window.devicePixelRatio || 1, 2);

/* ---- Capability and tier ------------------------------------------------- */

function hasWebGL() {
  try {
    const canvas = document.createElement("canvas");
    return Boolean(canvas.getContext("webgl2") || canvas.getContext("webgl"));
  } catch (e) {
    return false;
  }
}

/** Tier 3 is the full rack; tier 1 drops the metal entirely and keeps the
 *  signal, because the lit layer is the expensive half and reads least on a
 *  narrow screen — while the signal is what carries the identity. */
function pickTier() {
  const w = window.innerWidth;
  let tier = w >= 1280 ? 3 : w >= 768 ? 2 : 1;
  if ((navigator.hardwareConcurrency || 8) <= 4) tier = Math.max(1, tier - 1);
  if ((navigator.deviceMemory || 8) <= 4) tier = Math.max(1, tier - 1);
  return tier;
}

const TIERS = {
  3: { panels: 48, particles: 2000, meters: 56, leds: 18, threeScale: 0.85 },
  2: { panels: 32, particles: 1200, meters: 36, leds: 12, threeScale: 0.75 },
  1: { panels: 0, particles: 520, meters: 20, leds: 8, threeScale: 0 },
};

/* ---- Shared layout model ------------------------------------------------- */

const rackLines = [];

function computeRackLines() {
  rackLines.length = 0;
  // Seven seams, inset from the edges so the outermost never sits under the
  // sticky header or the footer rule.
  for (let i = 0; i < 7; i++) rackLines.push(0.08 + (i / 6) * 0.84);
}

/* ---- Entry --------------------------------------------------------------- */

export async function initBackground() {
  if (!hasWebGL()) return; // The CSS fallback composition is already showing.

  const tier = pickTier();
  const config = TIERS[tier];
  computeRackLines();

  const shared = {
    time: 0,
    scroll: 0,
    pointer: { x: 0, y: 0 },
    tally: new Float32Array(Math.max(config.panels, 1)),
    dark: document.documentElement.dataset.theme === "dark",
  };

  const layers = [];

  // Both renderers are imported after first paint, and failure to build one
  // must never take the other — or the page — down with it.
  try {
    if (config.panels > 0) layers.push(await buildMetal(config, shared));
  } catch (error) {
    console.warn("[scenedeck] metal layer unavailable:", error);
  }

  try {
    layers.push(await buildSignal(config, shared));
  } catch (error) {
    console.warn("[scenedeck] signal layer unavailable:", error);
  }

  if (!layers.length) return;

  // Theme and accent are page state, not animation state, so they are wired up
  // for everyone — including the reduced-motion path, which otherwise renders
  // one frame in its boot palette and then silently disagrees with the rest of
  // the page for the whole visit.
  bindAppearance(layers, shared);

  // A single frame, then stop: the full composition with none of the motion.
  if (reducedMotion.matches) {
    const paint = () => layers.forEach((layer) => layer.render(0));
    paint();
    layers.forEach((layer) => layer.canvas.setAttribute("data-live", ""));
    window.addEventListener("appearance:repaint", paint);
    window.addEventListener(
      "resize",
      debounce(() => {
        computeRackLines();
        layers.forEach((layer) => layer.resize());
        paint();
      }, 160)
    );
    return;
  }

  drive(layers, shared);
}

/** Theme and accent listeners, shared by the animated and static paths. */
function bindAppearance(layers, shared) {
  const repaint = () => window.dispatchEvent(new CustomEvent("appearance:repaint"));

  window.addEventListener("scenedeck:theme", (event) => {
    shared.dark = event.detail.dark;
    for (const layer of layers) layer.theme?.(event.detail.dark);
    repaint();
  });

  window.addEventListener("scenedeck:accent", (event) => {
    for (const layer of layers) layer.accent?.(event.detail.hex);
    repaint();
  });
}

/* ---- The driver ---------------------------------------------------------- */
/* One requestAnimationFrame for the entire page. It pauses outright when the
   tab is hidden or the window loses focus — both renderers then cost nothing
   at all rather than idling. */

function drive(layers, shared) {
  let raf = 0;
  let last = 0;
  let running = false;

  const frame = (now) => {
    raf = requestAnimationFrame(frame);
    const dt = Math.min((now - last) / 1000, 0.0333);
    last = now;
    shared.time += dt;

    shared.scroll = frameState.progress;
    // Delta-normalised damping, so the follow speed is the same at 60Hz and
    // 144Hz rather than three times faster on the better monitor.
    const k = 1 - Math.pow(1 - 0.06, dt * 60);
    shared.pointer.x += (frameState.pointerX - shared.pointer.x) * k;
    shared.pointer.y += (frameState.pointerY - shared.pointer.y) * k;

    for (let i = 0; i < shared.tally.length; i++) {
      if (shared.tally[i] > 0) shared.tally[i] = Math.max(0, shared.tally[i] - dt * 1.6);
    }

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
      computeRackLines();
      for (const layer of layers) layer.resize();
    }, 120)
  );

  // A program take runs the tally lamps across an arc of panels, staggered.
  window.addEventListener("scenedeck:take", () => {
    const n = shared.tally.length;
    const start = Math.floor(Math.random() * Math.max(1, n - 6));
    for (let i = 0; i < 6 && start + i < n; i++) {
      setTimeout(() => {
        shared.tally[start + i] = 1;
      }, i * 18);
    }
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

const readVar = (name) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim();

/* ============================================================================
   THE METAL — three.js
   ========================================================================== */

async function buildMetal(config, shared) {
  const THREE = await import("../vendor/three.min.js");
  const canvas = document.getElementById("bg-three");

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    powerPreference: "high-performance",
    stencil: false,
  });
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = shared.dark ? 1.05 : 0.92;

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(32, 1, 0.1, 40);
  camera.position.set(0, 0, 8.2);

  scene.fog = new THREE.FogExp2(new THREE.Color(readVar("--md-surface") || "#0e0c13"), 0.055);

  // A rounded rack panel, extruded with a small bevel so its edges catch light.
  const shape = roundedRect(THREE, 3.2, 0.62, 0.11);
  const geometry = new THREE.ExtrudeGeometry(shape, {
    depth: 0.14,
    bevelEnabled: true,
    bevelSegments: 2,
    bevelSize: 0.012,
    bevelThickness: 0.012,
    curveSegments: 6,
  });
  geometry.center();

  const uniforms = {
    uTime: { value: 0 },
    uBrush: { value: 0.045 },
    uAnodize: { value: new THREE.Color(shared.dark ? "#6f78ff" : "#4a4cbb") },
    uTallyColor: { value: new THREE.Color("#ed333b") },
    uPointer: { value: new THREE.Vector2(0, 0) },
  };

  const material = new THREE.MeshStandardMaterial({
    metalness: 0.86,
    roughness: 0.34,
    color: new THREE.Color(shared.dark ? "#6f6a80" : "#b9b4c4"),
    envMapIntensity: 1.15,
  });

  material.onBeforeCompile = (shader) => {
    Object.assign(shader.uniforms, uniforms);

    shader.vertexShader = shader.vertexShader
      .replace(
        "#include <common>",
        `#include <common>
         attribute float aPhase;
         attribute float aTally;
         attribute float aBand;
         varying float vPhase;
         varying float vTally;
         varying vec2 vPanelUv;
         uniform float uTime;
         uniform vec2 uPointer;`
      )
      .replace(
        "#include <begin_vertex>",
        `#include <begin_vertex>
         vPhase = aPhase;
         vTally = aTally;
         vPanelUv = vec2((position.x + 1.6) / 3.2, (position.y + 0.31) / 0.62);
         float w = uTime * 0.28 + aPhase;
         transformed.z += sin(w) * 0.09;
         transformed.y += cos(w * 0.61) * 0.035;
         float tilt = sin(w * 0.37) * 0.03 + aTally * 0.14;
         float cs = cos(tilt);
         float sn = sin(tilt);
         transformed.xz = mat2(cs, -sn, sn, cs) * transformed.xz;`
      )
      // Per-band pointer parallax, applied after the instance matrix so it is
      // a world-space offset rather than something the instance's own scale
      // distorts. Doing it here — instead of with one group per band — is what
      // buys genuine z-separation while the whole wall stays a single draw
      // call.
      .replace(
        "#include <project_vertex>",
        `vec4 mvPosition = vec4( transformed, 1.0 );
         #ifdef USE_INSTANCING
           mvPosition = instanceMatrix * mvPosition;
         #endif
         mvPosition.xy += uPointer * vec2(0.62, -0.44) * (0.25 + aBand * 0.375);
         mvPosition = modelViewMatrix * mvPosition;
         gl_Position = projectionMatrix * mvPosition;`
      );

    shader.fragmentShader = shader.fragmentShader
      .replace(
        "#include <common>",
        `#include <common>
         varying float vPhase;
         varying float vTally;
         varying vec2 vPanelUv;
         uniform float uBrush;
         uniform vec3 uAnodize;
         uniform vec3 uTallyColor;`
      )
      // Anisotropic brushed metal. The grooves run along the panel, so the
      // normal is perturbed on Y as a function of the panel's own U — three
      // octaves is enough to stop it banding.
      .replace(
        "#include <normal_fragment_maps>",
        `#include <normal_fragment_maps>
         float bu = vPanelUv.x;
         float brush = sin(bu * 420.0 + vPhase * 6.283) * 0.50
                     + sin(bu * 1130.0 + vPhase * 2.700) * 0.28
                     + sin(bu * 2600.0) * 0.12;
         normal = normalize(normal + vec3(0.0, brush * uBrush, 0.0));
         float fres = pow(1.0 - clamp(dot(normalize(vViewPosition), normal), 0.0, 1.0), 3.0);
         diffuseColor.rgb = mix(diffuseColor.rgb, uAnodize, fres * 0.55);`
      )
      // The tally lamp sits in the right-hand 18% of the panel — a real tally
      // position on real hardware.
      .replace(
        "#include <emissivemap_fragment>",
        `#include <emissivemap_fragment>
         totalEmissiveRadiance += uTallyColor * vTally * (0.35 + 0.65 * step(0.82, vPanelUv.x));`
      );
  };

  // Three depth bands, each in its own group so pointer parallax can move them
  // by different amounts — genuine z-separation rather than a uniform tilt.
  const BANDS = [
    { z: -3.5, scale: 0.72, parallax: 0.25 },
    { z: -1.0, scale: 1.0, parallax: 0.6 },
    { z: 1.4, scale: 1.34, parallax: 1.0 },
  ];
  const split = [0.42, 0.37, 0.21];

  const mesh = new THREE.InstancedMesh(geometry, material, config.panels);
  mesh.frustumCulled = false;

  const phases = new Float32Array(config.panels);
  const tallies = new Float32Array(config.panels);
  const dummy = new THREE.Object3D();
  const bands = new Float32Array(config.panels);

  // Panels are placed against the actual view frustum rather than in arbitrary
  // world units, so the wall fills the screen at any aspect ratio instead of
  // drifting off the sides of a wide monitor or emptying out on a narrow one.
  const layoutPanels = () => {
    const aspect = window.innerWidth / Math.max(1, window.innerHeight);
    const halfFov = (camera.fov * Math.PI) / 360;
    let index = 0;

    BANDS.forEach((band, b) => {
      const count =
        b === BANDS.length - 1 ? config.panels - index : Math.round(config.panels * split[b]);

      // What this band's depth can actually show.
      const distance = camera.position.z - band.z;
      const visibleH = Math.tan(halfFov) * distance;
      const visibleW = visibleH * aspect;

      // Panels are sized as a fraction of the visible width rather than in
      // fixed world units, so the wall reads at the same density on a phone
      // and on an ultrawide. A panel is background texture, not a slab: about
      // a fifth of the screen wide at the nearest band.
      const scale = ((visibleW * 2 * 0.17) / 3.2) * band.scale;
      const columns = Math.max(3, Math.ceil(count / rackLines.length));

      for (let i = 0; i < count && index < config.panels; i++, index++) {
        const row = i % rackLines.length;
        const column = Math.floor(i / rackLines.length) % columns;
        const jitter = Math.sin(index * 12.9898) * 0.3;

        // Spread a little past the edges so panels keep entering and leaving
        // rather than stopping at a visible boundary.
        const spread = visibleW * 1.3;
        dummy.position.set(
          -spread + ((column + 0.5 + jitter) / columns) * spread * 2,
          (0.5 - rackLines[row]) * visibleH * 2.3 + Math.cos(index * 78.233) * 0.12,
          band.z
        );
        dummy.rotation.set(0, Math.sin(index * 4.1) * 0.06, 0);
        dummy.scale.setScalar(scale);
        dummy.updateMatrix();
        mesh.setMatrixAt(index, dummy.matrix);
        phases[index] = (index * 2.399963) % 6.283185;
        bands[index] = b;
      }
    });

    // Matrices are written here and nowhere else. Every per-frame motion
    // happens in the vertex shader from aPhase + uTime, so there is no
    // per-frame CPU matrix work.
    mesh.instanceMatrix.needsUpdate = true;
  };

  layoutPanels();
  geometry.setAttribute("aPhase", new THREE.InstancedBufferAttribute(phases, 1));
  geometry.setAttribute("aBand", new THREE.InstancedBufferAttribute(bands, 1));
  const tallyAttr = new THREE.InstancedBufferAttribute(tallies, 1);
  geometry.setAttribute("aTally", tallyAttr);

  const group = new THREE.Group();
  group.add(mesh);
  scene.add(group);

  const key = new THREE.DirectionalLight(0xffffff, 1.6);
  key.position.set(4, 6, 5);
  scene.add(key);
  scene.add(new THREE.AmbientLight(0xffffff, shared.dark ? 0.18 : 0.5));

  // The environment is generated, not downloaded: a 256x128 gradient with two
  // elliptical softboxes. It is what produces the reflection that slides
  // across the panels as they drift, and it costs one texture at boot.
  const pmrem = new THREE.PMREMGenerator(renderer);
  const envTexture = new THREE.CanvasTexture(studioCanvas());
  envTexture.mapping = THREE.EquirectangularReflectionMapping;
  const env = pmrem.fromEquirectangular(envTexture).texture;
  scene.environment = env;
  envTexture.dispose();
  pmrem.dispose();

  const resize = () => {
    const w = window.innerWidth;
    const h = window.innerHeight;
    renderer.setPixelRatio(DPR() * config.threeScale);
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    layoutPanels();
  };
  resize();

  let tallyDirty = false;
  let accentHex = null;

  return {
    canvas,
    resize,
    theme(dark) {
      material.color.set(dark ? "#6f6a80" : "#b9b4c4");
      // A chosen scene accent outranks the theme default, otherwise toggling
      // the colour scheme would silently throw the accent away and leave the
      // metal disagreeing with the rest of the page.
      uniforms.uAnodize.value.set(accentHex ?? (dark ? "#6f78ff" : "#4a4cbb"));
      renderer.toneMappingExposure = dark ? 1.05 : 0.92;
      scene.fog.color.set(readVar("--md-surface") || (dark ? "#0e0c13" : "#faf9f8"));
    },
    accent(hex) {
      accentHex = hex;
      uniforms.uAnodize.value.set(hex);
    },
    render() {
      uniforms.uTime.value = shared.time;

      camera.position.z = 8.2 - shared.scroll * 2.8;
      camera.position.y = -shared.scroll * 1.6;
      camera.lookAt(0, -shared.scroll * 1.6, 0);

      // A small whole-wall tilt on top of the per-band offset the shader
      // applies, so the rack turns as well as separating.
      group.rotation.y = shared.pointer.x * 0.03;
      group.rotation.x = -shared.pointer.y * 0.02;
      uniforms.uPointer.value.set(shared.pointer.x, shared.pointer.y);

      // The tally attribute is only re-uploaded while something is actually
      // lit, so the common case costs nothing.
      let any = false;
      for (let i = 0; i < tallies.length; i++) {
        if (tallies[i] !== shared.tally[i]) {
          tallies[i] = shared.tally[i];
          tallyDirty = true;
        }
        if (tallies[i] > 0.001) any = true;
      }
      if (tallyDirty) {
        tallyAttr.needsUpdate = true;
        tallyDirty = any;
      }

      renderer.render(scene, camera);
    },
  };
}

function roundedRect(THREE, w, h, r) {
  const shape = new THREE.Shape();
  const x = -w / 2;
  const y = -h / 2;
  shape.moveTo(x + r, y);
  shape.lineTo(x + w - r, y);
  shape.quadraticCurveTo(x + w, y, x + w, y + r);
  shape.lineTo(x + w, y + h - r);
  shape.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  shape.lineTo(x + r, y + h);
  shape.quadraticCurveTo(x, y + h, x, y + h - r);
  shape.lineTo(x, y + r);
  shape.quadraticCurveTo(x, y, x + r, y);
  return shape;
}

/** The studio: a vertical gradient with two softboxes and a warm bounce. */
function studioCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 256;
  canvas.height = 128;
  const ctx = canvas.getContext("2d");

  const gradient = ctx.createLinearGradient(0, 0, 0, 128);
  gradient.addColorStop(0, "#ffffff");
  gradient.addColorStop(0.42, "#cfd2ff");
  gradient.addColorStop(0.55, "#3a3550");
  gradient.addColorStop(1, "#0a0810");
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, 256, 128);

  const softbox = (x, y, r, alpha) => {
    const glow = ctx.createRadialGradient(x, y, 0, x, y, r);
    glow.addColorStop(0, `rgba(255,255,255,${alpha})`);
    glow.addColorStop(1, "rgba(255,255,255,0)");
    ctx.fillStyle = glow;
    ctx.fillRect(x - r, y - r, r * 2, r * 2);
  };

  softbox(0.24 * 256, 0.22 * 128, 0.16 * 256, 0.9);
  softbox(0.78 * 256, 0.3 * 128, 0.11 * 256, 0.65);

  ctx.fillStyle = "rgba(255,190,111,0.35)";
  ctx.fillRect(0, 0.66 * 128, 256, 5);

  return canvas;
}

/* ============================================================================
   THE SIGNAL — pixi.js
   ========================================================================== */

async function buildSignal(config, shared) {
  const PIXI = await import("../vendor/pixi.min.js");
  const canvas = document.getElementById("bg-pixi");

  const renderer = new PIXI.WebGLRenderer();
  await renderer.init({
    canvas,
    width: window.innerWidth,
    height: window.innerHeight,
    backgroundAlpha: 0,
    antialias: false,
    resolution: config.threeScale ? DPR() : 1,
    autoDensity: true,
    powerPreference: "high-performance",
  });

  const stage = new PIXI.Container();

  const dotTexture = PIXI.Texture.from(dotCanvas());
  const pxTexture = PIXI.Texture.from(pixelCanvas());

  const tint = () => Number(`0x${(readVar("--bg-canvas-tint") || "#8a90ff").replace("#", "")}`);
  let currentTint = tint();

  // Telemetry flowing along the seams between panel rows. One container, one
  // texture, one draw call.
  const flow = new PIXI.ParticleContainer({
    dynamicProperties: { position: true, color: true, vertex: false, rotation: false },
  });
  const particles = [];

  for (let i = 0; i < config.particles; i++) {
    const bus = i % rackLines.length;
    const particle = new PIXI.Particle({
      texture: dotTexture,
      x: Math.random() * window.innerWidth,
      y: 0,
      tint: currentTint,
      alpha: [0.22, 0.38, 0.55][bus % 3],
    });
    const scale = 0.18 + Math.random() * 0.26;
    particle.scaleX = scale;
    particle.scaleY = scale;
    flow.addParticle(particle);
    particles.push({
      p: particle,
      bus,
      speed: 18 + (bus / rackLines.length) * 34 + Math.random() * 8,
      seed: Math.random() * 6.283,
    });
  }

  // The VU field. All segments share one 1x1 texture, so pixi batches the
  // whole wall into a single draw.
  const meters = new PIXI.ParticleContainer({
    dynamicProperties: { position: false, color: true, vertex: false, rotation: false },
  });
  const meterCells = [];
  const SEGMENTS = 6;

  for (let m = 0; m < config.meters; m++) {
    for (let s = 0; s < SEGMENTS; s++) {
      const particle = new PIXI.Particle({ texture: pxTexture, x: 0, y: 0, tint: currentTint, alpha: 0 });
      particle.scaleX = 5;
      particle.scaleY = 9;
      meters.addParticle(particle);
      meterCells.push({ p: particle, meter: m, seg: s });
    }
  }

  // Tally lamps sitting on the seams. The one nearest the section in view
  // ignites; the rest stay at a resting glow.
  const leds = new PIXI.ParticleContainer({
    dynamicProperties: { position: false, color: true, vertex: false, rotation: false },
  });
  const ledList = [];
  for (let i = 0; i < config.leds; i++) {
    const particle = new PIXI.Particle({ texture: pxTexture, x: 0, y: 0, tint: 0x8e8f9a, alpha: 0.12 });
    particle.scaleX = 7;
    particle.scaleY = 7;
    leds.addParticle(particle);
    ledList.push(particle);
  }

  stage.addChild(flow, meters, leds);

  const applyBlend = (dark) => {
    const mode = dark ? "add" : "normal";
    flow.blendMode = mode;
    meters.blendMode = mode;
    leds.blendMode = mode;
    const alphaScale = dark ? 1 : 0.55;
    for (const entry of particles) entry.p.alpha = [0.22, 0.38, 0.55][entry.bus % 3] * alphaScale;
  };
  applyBlend(shared.dark);

  let W = window.innerWidth;
  let H = window.innerHeight;

  const layout = () => {
    W = window.innerWidth;
    H = window.innerHeight;

    // Meters sit along the bottom seam, evenly spread.
    const meterGap = W / config.meters;
    for (const cell of meterCells) {
      cell.p.x = cell.meter * meterGap + meterGap * 0.3;
      cell.p.y = H * rackLines[rackLines.length - 1] - cell.seg * 12;
    }

    ledList.forEach((led, i) => {
      led.x = ((i * 137.5) % 100) * (W / 100);
      led.y = H * rackLines[i % rackLines.length];
    });

    // Both containers hold position static, so pixi only uploads their vertex
    // buffer when it is told the children changed. Without this the meters and
    // lamps stay at their pre-resize coordinates.
    meters.update();
    leds.update();
  };

  const resize = () => {
    renderer.resize(window.innerWidth, window.innerHeight);
    layout();
  };
  layout();

  // A fixed logical step, so the signal runs at the same speed on a 144Hz
  // display as on a 60Hz one.
  let accumulator = 0;
  const STEP = 1 / 60;
  const envelopes = new Float32Array(config.meters);

  const stepLogic = (t) => {
    for (const entry of particles) {
      entry.p.x += entry.speed * STEP;
      if (entry.p.x > W + 8) entry.p.x = -8;
      entry.p.y = H * rackLines[entry.bus] + Math.sin(t * 2.1 + entry.seed) * 3;
    }

    for (let m = 0; m < config.meters; m++) {
      const target =
        0.5 + 0.28 * Math.sin(t * 1.7 + m * 0.41) + 0.14 * Math.sin(t * 4.3 + m * 1.13);
      // Fast attack, slow release. That asymmetry is what makes a meter read
      // as audio rather than as a sine wave.
      const rate = target > envelopes[m] ? 0.18 : 0.045;
      envelopes[m] += (target - envelopes[m]) * rate;
    }

    for (const cell of meterCells) {
      const level = envelopes[cell.meter];
      const on = cell.seg < Math.round(level * SEGMENTS);
      cell.p.alpha = on ? 0.75 : 0.08;
      cell.p.tint = !on
        ? 0x494252
        : level > 0.88
          ? 0xed333b
          : level > 0.72
            ? 0xffbe6f
            : currentTint;
    }

    const lit = Math.floor(shared.scroll * ledList.length);
    ledList.forEach((led, i) => {
      const active = i === lit;
      led.alpha += ((active ? 0.95 : 0.12) - led.alpha) * 0.12;
      led.tint = active ? 0xed333b : 0x8e8f9a;
    });
  };

  // Seed one logic pass so the signal is composed before the first render.
  // Without it the reduced-motion path — which renders a single frame and then
  // never ticks — would draw every particle stacked at the origin.
  for (const entry of particles) entry.p.x = Math.random() * window.innerWidth;
  stepLogic(0);

  return {
    canvas,
    resize,
    theme(dark) {
      currentTint = tint();
      for (const entry of particles) entry.p.tint = currentTint;
      applyBlend(dark);
    },
    accent(hex) {
      currentTint = Number(`0x${hex.replace("#", "")}`);
      for (const entry of particles) entry.p.tint = currentTint;
    },
    render(dt) {
      accumulator += dt;
      let guard = 0;
      while (accumulator >= STEP && guard++ < 4) {
        accumulator -= STEP;
        stepLogic(shared.time);
      }
      renderer.render(stage);
    },
  };
}

/** A feathered dot. Pre-baking the falloff is what lets the additive blend
 *  look like bloom without a filter pass. */
function dotCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 16;
  canvas.height = 16;
  const ctx = canvas.getContext("2d");
  const glow = ctx.createRadialGradient(8, 8, 0, 8, 8, 8);
  glow.addColorStop(0, "rgba(255,255,255,1)");
  glow.addColorStop(0.4, "rgba(255,255,255,0.55)");
  glow.addColorStop(0.7, "rgba(255,255,255,0.18)");
  glow.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, 16, 16);
  return canvas;
}

function pixelCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, 1, 1);
  return canvas;
}
