/* ============================================================================
   The wire — the model both renderers share.
   ============================================================================

   SceneDeck's whole premise is two machines and one link between them, so the
   background is that link seen from inside: a channel receding to a vanishing
   point, eight bit-lanes running along its wall, and encoded video travelling
   up it toward the camera.

   This module is the single source of truth for the channel's geometry. The
   three.js shader draws the channel wall from these constants and pixi places
   every packet with the same projection, so a packet on lane 3 at z = 12 lands
   exactly on the rail for lane 3 at z = 12 — at any aspect ratio, at any
   pointer position. `centreline()` exists twice, once here and once in GLSL,
   and the two copies must stay identical or the layers visibly separate.
   ========================================================================== */

export const wire = {
  /* Geometry. These numbers appear in the shader as literals. */
  FOCAL: 1.7320508075688772, // 1 / tan(60deg / 2)
  RADIUS: 1.35,
  LANES: 8,
  Z_NEAR: 0.55,
  Z_FAR: 40,

  /* OBS's default keyframe interval. The GOP clock runs on it, so the whole
     background has a two-second heartbeat that is a real OBS number. */
  GOP_MS: 2000,

  /* Per-frame state. Written once per frame by the driver, read by both
     layers. Nothing else may write these. */
  vpX: 0.3,
  vpY: -0.02,
  pointerX: 0,
  pointerY: 0,
  scroll: 0,
  bitrate: 0.62,
  congestion: 0.1,
  keyZ: 40,
  takeEnergy: 0,
  accentMix: 0,
  aspect: 1,
  time: 0,
};

/**
 * Where the axis of the channel sits at depth z. Drifts slowly on its own and
 * leans toward the pointer, weighted by depth so the far end swings more than
 * the near end — which is what sells it as a long tube rather than a texture.
 *
 * MUST match centreline() in the fragment shader exactly.
 */
export function centreline(z, t, px, py, out) {
  const k = z * 0.0125;
  out.x = 0.62 * Math.sin(z * 0.06 + t * 0.15) + px * k;
  out.y = 0.44 * Math.cos(z * 0.045 + t * 0.11) + py * k;
  return out;
}

/**
 * Project a point on the channel wall to normalised screen space.
 *
 * The three.js side gets this for free from the ray direction; pixi has no
 * camera, so it reproduces the same projection here. Returns false when the
 * point is behind or too close to the eye to be meaningful.
 */
const _c = { x: 0, y: 0 };

export function project(lane, radius, z, out) {
  if (z <= 0.05) return false;

  const angle = ((lane + 0.5) / wire.LANES) * Math.PI * 2 - Math.PI;
  centreline(z, wire.time, wire.pointerX, wire.pointerY, _c);

  const x = Math.cos(angle) * wire.RADIUS * radius + _c.x;
  const y = Math.sin(angle) * wire.RADIUS * radius + _c.y;

  // Perspective divide, then the same NDC shear the shader applies to move
  // the vanishing point off centre.
  const inv = wire.FOCAL / z;
  out.ndcX = (x * inv) / wire.aspect + wire.vpX;
  out.ndcY = y * inv + wire.vpY;
  out.scale = inv;
  return Math.abs(out.ndcX) < 2.4 && Math.abs(out.ndcY) < 2.4;
}
