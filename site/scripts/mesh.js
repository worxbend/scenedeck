/* ============================================================================
   The mesh — the layout both renderers share.
   ============================================================================

   A triangulated dataflow network: nodes on a jittered triangular lattice,
   edges between neighbours, and packets travelling those edges.

   The lattice is the reason it triangulates cleanly. Points sit on a
   triangular grid — every other row offset by half a cell — and each point
   links right, down-right and down-left. Those three links per point are
   exactly the edges of a triangle, so the result is a real triangulation
   rather than a nearest-neighbour approximation that leaves stray quads and
   crossings.

   Everything here is in CSS pixels, not world units. The renderers convert at
   the last moment. Working in pixels means the spacing, line weight and packet
   speed are the numbers they claim to be at any DPR or aspect, and it removes
   the aspect-divide that a world-space model needs — one less place for the
   two layers to disagree.
   ========================================================================== */

export const mesh = {
  width: 0,
  height: 0,
  time: 0,
  scroll: 0,
  pointerX: 0,
  pointerY: 0,

  /* Depth by parallax rate, not by projection. The near layer is sparser and
     brighter so it does not smother the layers behind it. */
  layers: [
    { cell: 132, parallax: 5, dim: 0.5, jitter: 0.3 },
    { cell: 108, parallax: 13, dim: 0.78, jitter: 0.34 },
    { cell: 95, parallax: 26, dim: 1, jitter: 0.38 },
  ],

  nodes: [],
  edges: [],
};

/** Deterministic hash, so the mesh is identical on every load. */
function rnd(i, salt) {
  const x = Math.sin(i * 127.1 + salt * 311.7) * 43758.5453;
  return x - Math.floor(x);
}

/**
 * Build the lattice.
 *
 * Overscans by two cells on every side so parallax and scroll can move a layer
 * without revealing an edge where the mesh simply stops.
 */
export function layout(width, height, budget) {
  mesh.width = width;
  mesh.height = height;
  mesh.nodes.length = 0;
  mesh.edges.length = 0;

  let seed = 0;

  mesh.layers.forEach((layer, li) => {
    const cell = layer.cell;
    const rowH = cell * 0.866; // equilateral: height of a triangular row
    const cols = Math.ceil(width / cell) + 4;
    const rows = Math.ceil(height / rowH) + 4;
    const first = mesh.nodes.length;
    const index = [];

    for (let r = 0; r < rows; r++) {
      index.push([]);
      for (let c = 0; c < cols; c++) {
        seed++;
        if (mesh.nodes.length >= budget) {
          index[r].push(-1);
          continue;
        }

        // Odd rows shift half a cell: that offset is what makes the lattice
        // triangular rather than square.
        const ox = (r % 2) * cell * 0.5;
        index[r].push(mesh.nodes.length);
        mesh.nodes.push({
          x: (c - 2) * cell + ox + (rnd(seed, 1) - 0.5) * cell * layer.jitter,
          y: (r - 2) * rowH + (rnd(seed, 2) - 0.5) * rowH * layer.jitter,
          layer: li,
          // A few nodes are junctions: drawn larger, and the only ones packets
          // are allowed to change direction at.
          hub: rnd(seed, 3) < 0.14,
        });
      }
    }

    // Right, down-right and down-left. Three edges per node closes the
    // triangles without emitting any edge twice.
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        const a = index[r][c];
        if (a < 0) continue;
        const odd = r % 2;
        const right = index[r][c + 1];
        const down = index[r + 1] ? index[r + 1][c] : undefined;
        const downSide = index[r + 1] ? index[r + 1][c + (odd ? 1 : -1)] : undefined;

        for (const b of [right, down, downSide]) {
          if (b === undefined || b < 0 || b < first) continue;
          // A tenth of the lattice is dropped so it reads as a network that
          // was routed rather than as graph paper.
          seed++;
          if (rnd(seed, 4) < 0.1) continue;
          mesh.edges.push({ a, b, layer: li });
        }
      }
    }
  });
}

/** A node's position after parallax and scroll, in pixels. */
export function nodeAt(node, out) {
  const l = mesh.layers[node.layer];
  out.x = node.x + mesh.pointerX * l.parallax;
  out.y = node.y - mesh.pointerY * l.parallax * 0.6 - mesh.scroll * l.parallax * 8;
  return out;
}

/** Pixels -> clip space. The only projection either renderer uses. */
export function toClip(x, y, out) {
  out.x = (x / mesh.width) * 2 - 1;
  out.y = 1 - (y / mesh.height) * 2;
  return out;
}
