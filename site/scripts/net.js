/* ============================================================================
   The network — the layout both renderers share.
   ============================================================================

   A circuit board carrying the link between two machines: pads at the
   junctions, traces routed between them the way a PCB routes them (a straight
   run, a 45-degree turn, another straight run), and data moving along those
   traces as packets and byte streams.

   Everything is placed in one flat space — x spans [-aspect, +aspect], y spans
   [-1, +1] — with no perspective divide anywhere. Three parallax layers give
   depth by moving at different rates rather than by projection. That matters:
   a trace is a straight line and a pad is a square, so nothing here can
   develop the seams and broken curves that a polar-coordinate background does.

   three.js draws the traces and the pads from this model; pixi draws what is
   travelling along them. `toScreen()` is the only projection either uses, so a
   packet at t=0.5 along trace 7 sits exactly halfway down the trace three.js
   drew.
   ========================================================================== */

export const net = {
  aspect: 1,
  time: 0,
  scroll: 0,
  pointerX: 0,
  pointerY: 0,

  /* The near layer moves most. That difference is the whole depth cue. */
  layers: [
    { parallax: 0.018, dim: 0.4, width: 0.0022 },
    { parallax: 0.045, dim: 0.7, width: 0.0032 },
    { parallax: 0.085, dim: 1.0, width: 0.0044 },
  ],

  nodes: [],
  traces: [],
};

/** Deterministic hash, so the board is identical on every load. */
function rnd(i, salt) {
  const x = Math.sin(i * 127.1 + salt * 311.7) * 43758.5453;
  return x - Math.floor(x);
}

/**
 * Lay out pads on a jittered grid and route traces between adjacent columns.
 *
 * Rebuilt on resize, because the column spread follows aspect: an ultrawide
 * gets more board rather than a stretched copy of the same board.
 */
export function layout(aspect, budget) {
  net.aspect = aspect;
  net.nodes.length = 0;
  net.traces.length = 0;

  const cols = 9;
  const rows = 8;
  const spanX = Math.min(aspect, 2.6) * 1.15;
  let i = 0;

  for (let c = 0; c < cols; c++) {
    for (let r = 0; r < rows; r++) {
      i++;
      // Gaps keep it reading as a routed board rather than a lattice, and
      // leave clear areas for copy to sit over.
      if (rnd(i, 3) < 0.22) continue;
      if (net.nodes.length >= budget) break;

      net.nodes.push({
        x: -spanX + (c / (cols - 1)) * spanX * 2 + (rnd(i, 1) - 0.5) * 0.12,
        y: -1.15 + (r / (rows - 1)) * 2.3 + (rnd(i, 2) - 0.5) * 0.14,
        layer: i % 3,
        column: c,
        // A few pads are hubs: bigger, with a ring. Those read as devices.
        hub: rnd(i, 4) < 0.26,
        phase: rnd(i, 5) * 6.283,
      });
    }
  }

  // Route each pad to one in the next column on the same layer. Signal runs
  // left to right, the way a board is usually read.
  net.nodes.forEach((node, index) => {
    const targets = net.nodes.filter(
      (n) => n.column === node.column + 1 && n.layer === node.layer
    );
    if (!targets.length) return;

    let best = targets[0];
    let bestD = Infinity;
    for (const t of targets) {
      const d = Math.abs(t.y - node.y);
      if (d < bestD) {
        bestD = d;
        best = t;
      }
    }
    net.traces.push({ from: index, to: net.nodes.indexOf(best), layer: node.layer });
  });
}

/** A pad's position after parallax and scroll. */
const _o = { x: 0, y: 0 };

export function offset(node, out) {
  const l = net.layers[node.layer];
  out.x = node.x + net.pointerX * l.parallax;
  out.y = node.y - net.pointerY * l.parallax * 0.55 - net.scroll * l.parallax * 4.2;
  return out;
}

/**
 * The four points of a trace's route: a straight run, a 45-degree turn, then
 * another straight run. The diagonal takes all of the vertical change, which
 * is exactly how a board is routed — and why every corner is either 90 or 45
 * degrees and never an arbitrary angle.
 */
export function tracePath(trace, out) {
  const a = net.nodes[trace.from];
  const b = net.nodes[trace.to];
  offset(a, _o);
  const ax = _o.x;
  const ay = _o.y;
  offset(b, _o);
  const bx = _o.x;
  const by = _o.y;

  const dx = bx - ax;
  const dy = by - ay;
  const sx = Math.sign(dx) || 1;
  // The 45-degree turn covers the vertical drop, but never more than a third
  // of the horizontal run — a board has short corners and long straights, and
  // letting the diagonal take everything produced near-vertical lines.
  const diagonal = Math.min(Math.abs(dy), Math.abs(dx) * 0.34);
  const run = (Math.abs(dx) - diagonal) * 0.5;

  out[0] = ax;
  out[1] = ay;
  out[2] = ax + run * sx;
  out[3] = ay;
  out[4] = out[2] + diagonal * sx;
  out[5] = by;
  out[6] = bx;
  out[7] = by;
  return out;
}

export function pathLength(p) {
  return (
    Math.hypot(p[2] - p[0], p[3] - p[1]) +
    Math.hypot(p[4] - p[2], p[5] - p[3]) +
    Math.hypot(p[6] - p[4], p[7] - p[5])
  );
}

/** Point at distance `d` along a trace, plus the direction it is travelling. */
export function pathPoint(p, d, out) {
  for (let s = 0; s < 3; s++) {
    const x0 = p[s * 2];
    const y0 = p[s * 2 + 1];
    const x1 = p[s * 2 + 2];
    const y1 = p[s * 2 + 3];
    const len = Math.hypot(x1 - x0, y1 - y0);
    if (d <= len || s === 2) {
      const t = len > 0 ? Math.min(1, d / len) : 0;
      out.x = x0 + (x1 - x0) * t;
      out.y = y0 + (y1 - y0) * t;
      out.dx = len > 0 ? (x1 - x0) / len : 1;
      out.dy = len > 0 ? (y1 - y0) / len : 0;
      return out;
    }
    d -= len;
  }
  return out;
}

/** World space -> pixel space. The only projection either renderer uses. */
export function toScreen(x, y, width, height, out) {
  out.x = ((x / net.aspect) * 0.5 + 0.5) * width;
  out.y = (0.5 - y * 0.5) * height;
  return out;
}
