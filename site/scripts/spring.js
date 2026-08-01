/* ============================================================================
   Material 3 Expressive motion physics.
   ============================================================================

   M3 Expressive replaced duration-and-curve animation with a spring system.
   The character comes from the damping ratio: the spatial springs sit at 0.6,
   under-damped, so they overshoot the target and settle back. That overshoot
   is the whole point — it is what makes motion read as physical rather than
   as a timed interpolation.

   The values below are Material's own ExpressiveMotionTokens, read from the
   androidx source rather than from a summary — several write-ups on the web
   quote the Standard scheme's stiffnesses next to Expressive's damping, which
   is a different, stiffer curve:

     compose/material3/.../tokens/ExpressiveMotionTokens.kt

     spatial  fast     stiffness  800  damping ratio 0.6
     spatial  default  stiffness  380  damping ratio 0.8
     spatial  slow     stiffness  200  damping ratio 0.8
     effects  fast     stiffness 3800  damping ratio 1.0
     effects  default  stiffness 1600  damping ratio 1.0
     effects  slow     stiffness  800  damping ratio 1.0

   For contrast, Standard runs its spatial springs at damping 0.9 and roughly
   double the stiffness — tighter, quicker, no real overshoot. Expressive is
   deliberately looser, and that is the whole personality of the scheme.

   Spatial springs move things: position, scale, rotation, shape. Effects
   springs change colour and opacity, and are critically damped at 1.0 because
   a colour that overshoots reads as a bug rather than as energy.

   Damping coefficient for unit mass is c = 2 * ratio * sqrt(stiffness).
   ========================================================================== */

export const SPRING = {
  spatialFast: { stiffness: 800, damping: 0.6 },
  spatialDefault: { stiffness: 380, damping: 0.8 },
  spatialSlow: { stiffness: 200, damping: 0.8 },
  effectsFast: { stiffness: 3800, damping: 1 },
  effectsDefault: { stiffness: 1600, damping: 1 },
  effectsSlow: { stiffness: 800, damping: 1 },
};

/**
 * A single scalar driven by a spring.
 *
 * Integration is semi-implicit Euler, substepped at a fixed 1/240s. A stiff
 * spring (effects fast is 3800) will diverge if it is integrated at whatever
 * the display happens to be doing, and the failure mode is an explosion to
 * NaN rather than a slightly wrong curve — so the step size is fixed and the
 * frame delta is consumed in whole substeps.
 */
export class Spring {
  constructor(value = 0, token = SPRING.spatialDefault) {
    this.value = value;
    this.target = value;
    this.velocity = 0;
    this.stiffness = token.stiffness;
    this.ratio = token.damping;
  }

  /** Retarget without discarding momentum — the point of a spring system. */
  to(target) {
    this.target = target;
    return this;
  }

  /** Jump to a value, killing motion. For init and for resize. */
  set(value) {
    this.value = value;
    this.target = value;
    this.velocity = 0;
    return this;
  }

  get settled() {
    return Math.abs(this.value - this.target) < 1e-4 && Math.abs(this.velocity) < 1e-3;
  }

  /** @param {number} dt seconds since the previous frame. */
  step(dt) {
    if (this.settled) {
      this.value = this.target;
      this.velocity = 0;
      return this.value;
    }

    const c = 2 * this.ratio * Math.sqrt(this.stiffness);
    // Cap the catch-up so a long tab-switch cannot spend a second of CPU
    // integrating history nobody saw.
    let remaining = Math.min(dt, 0.1);
    const H = 1 / 240;

    while (remaining > 0) {
      const h = Math.min(H, remaining);
      remaining -= h;
      const accel = -this.stiffness * (this.value - this.target) - c * this.velocity;
      this.velocity += accel * h;
      this.value += this.velocity * h;
    }

    return this.value;
  }
}

/** Two springs sharing a token, for pointer and other 2-vectors. */
export class Spring2 {
  constructor(x = 0, y = 0, token = SPRING.spatialDefault) {
    this.x = new Spring(x, token);
    this.y = new Spring(y, token);
  }

  to(x, y) {
    this.x.to(x);
    this.y.to(y);
    return this;
  }

  set(x, y) {
    this.x.set(x);
    this.y.set(y);
    return this;
  }

  step(dt) {
    this.x.step(dt);
    this.y.step(dt);
    return this;
  }
}
