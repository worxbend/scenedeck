/* ============================================================================
   OKLab / OKLCH conversion.
   ============================================================================
   Material 3's dynamic colour works by holding a hue and chroma fixed and
   walking a lightness ladder — so the site needs a perceptual colour space to
   do it honestly rather than by nudging HSL and hoping. The constants are
   Björn Ottosson's.

   Everything here is pure. Give it numbers, get numbers back.
   ========================================================================== */

const clamp01 = (n) => (n < 0 ? 0 : n > 1 ? 1 : n);

function srgbToLinear(c) {
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function linearToSrgb(c) {
  return c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055;
}

/** #rrggbb -> {L, C, H} with H in degrees. */
export function hexToOklch(hex) {
  const int = parseInt(hex.replace("#", ""), 16);
  const r = srgbToLinear(((int >> 16) & 255) / 255);
  const g = srgbToLinear(((int >> 8) & 255) / 255);
  const b = srgbToLinear((int & 255) / 255);

  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);

  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const bb = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;

  return {
    L,
    C: Math.hypot(a, bb),
    // Normalised to 0..360. atan2 returns -180..180, and while CSS accepts a
    // negative hue angle it is confusing to read in devtools.
    H: (((Math.atan2(bb, a) * 180) / Math.PI) + 360) % 360,
  };
}

/** {L, C, H} -> #rrggbb, clipped into sRGB by reducing chroma. */
export function oklchToHex(L, C, H) {
  const rad = (H * Math.PI) / 180;

  // Out-of-gamut colours are pulled back by desaturating rather than by
  // clipping each channel, which would shift the hue.
  for (let chroma = C; chroma >= 0; chroma -= 0.005) {
    const a = Math.cos(rad) * chroma;
    const b = Math.sin(rad) * chroma;

    const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
    const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
    const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;

    const r = linearToSrgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s);
    const g = linearToSrgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s);
    const bl = linearToSrgb(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s);

    const inGamut = [r, g, bl].every((v) => v >= -0.001 && v <= 1.001);
    if (inGamut || chroma <= 0) {
      const to255 = (v) => Math.round(clamp01(v) * 255);
      return `#${((to255(r) << 16) | (to255(g) << 8) | to255(bl)).toString(16).padStart(6, "0")}`;
    }
  }

  return "#000000";
}

/** Relative luminance, for contrast checks. */
export function luminance(hex) {
  const int = parseInt(hex.replace("#", ""), 16);
  const [r, g, b] = [(int >> 16) & 255, (int >> 8) & 255, int & 255].map((v) =>
    srgbToLinear(v / 255)
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrast(a, b) {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}
