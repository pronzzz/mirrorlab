import { Adjustments, Point } from '../types';

// Helper: Cubic Spline Interpolation for smooth curves
function splineInterpolate(points: Point[], xArr: number[]): Uint8Array {
  const lut = new Uint8Array(256);
  const sorted = [...points].sort((a, b) => a.x - b.x);

  for (let i = 0; i < 256; i++) {
    const x = i / 255;

    let p0 = sorted[0];
    let p1 = sorted[sorted.length - 1];

    for (let j = 0; j < sorted.length - 1; j++) {
      if (x >= sorted[j].x && x <= sorted[j + 1].x) {
        p0 = sorted[j];
        p1 = sorted[j + 1];
        break;
      }
    }

    const range = p1.x - p0.x;
    const t = range === 0 ? 0 : (x - p0.x) / range;
    const y = p0.y + t * (p1.y - p0.y);

    lut[i] = Math.max(0, Math.min(255, Math.floor(y * 255)));
  }
  return lut;
}

// Convert RGB to HSL
function rgbToHsl(r: number, g: number, b: number) {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  let h = 0, s, l = (max + min) / 2;

  if (max === min) {
    h = s = 0;
  } else {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r: h = (g - b) / d + (g < b ? 6 : 0); break;
      case g: h = (b - r) / d + 2; break;
      case b: h = (r - g) / d + 4; break;
    }
    h /= 6;
  }
  return [h, s, l];
}

// Convert HSL to RGB
function hslToRgb(h: number, s: number, l: number) {
  let r, g, b;

  if (s === 0) {
    r = g = b = l;
  } else {
    const hue2rgb = (p: number, q: number, t: number) => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };

    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;
    r = hue2rgb(p, q, h + 1 / 3);
    g = hue2rgb(p, q, h);
    b = hue2rgb(p, q, h - 1 / 3);
  }

  return [Math.round(r * 255), Math.round(g * 255), Math.round(b * 255)];
}

export const processImage = (
  ctx: CanvasRenderingContext2D,
  originalImageData: ImageData,
  adj: Adjustments
): void => {
  const width = originalImageData.width;
  const height = originalImageData.height;

  const outputImage = ctx.createImageData(width, height);
  const src = originalImageData.data;
  const dst = outputImage.data;

  // 1. Curve LUTs
  const lutMaster = splineInterpolate(adj.curve.master, []);
  const lutRed = splineInterpolate(adj.curve.red, []);
  const lutGreen = splineInterpolate(adj.curve.green, []);
  const lutBlue = splineInterpolate(adj.curve.blue, []);

  // 2. Pre-calc factors
  const contrastFactor = (259 * (adj.contrast + 255)) / (255 * (259 - adj.contrast));
  const exposureMultiplier = Math.pow(2, adj.exposure);

  const rBalance = 1 + (adj.temperature / 100);
  const bBalance = 1 - (adj.temperature / 100);
  const gBalance = 1 + (adj.tint / 100);

  // Clarity implementation: Local contrast enhancement (Simplified using a slight unsharp mask approach or overlay)
  // For high performance in JS, we'll use a fast approximation: boosting contrast based on mid-range luminance
  const clarityFactor = adj.clarity / 100;

  for (let i = 0; i < src.length; i += 4) {
    let r = src[i];
    let g = src[i + 1];
    let b = src[i + 2];

    // --- Basic Tone Pipeline ---

    // 1. Exposure
    r *= exposureMultiplier;
    g *= exposureMultiplier;
    b *= exposureMultiplier;

    // 2. White Balance
    r *= rBalance;
    g *= gBalance;
    b *= bBalance;

    // 3. Clarity (Simplified: Mid-tone contrast boost)
    if (clarityFactor !== 0) {
      const l = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
      // Weight centered around 0.5
      const weight = 1.0 - Math.abs(l - 0.5) * 2;
      const boost = 1 + (clarityFactor * weight * 0.5);
      r = (r - 128) * boost + 128;
      g = (g - 128) * boost + 128;
      b = (b - 128) * boost + 128;
    }

    // 4. Highlights / Shadows / Whites / Blacks
    const lum = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;

    if (adj.highlights !== 0) {
      const weight = Math.pow(Math.max(0, lum), 2);
      const factor = 1 + (adj.highlights / 100) * weight;
      r *= factor; g *= factor; b *= factor;
    }

    if (adj.shadows !== 0) {
      const weight = Math.pow(Math.max(0, 1 - lum), 2);
      const factor = 1 + (adj.shadows / 100) * weight;
      r *= factor; g *= factor; b *= factor;
    }

    if (adj.whites !== 0) {
      const weight = Math.pow(Math.max(0, lum), 4);
      const val = (adj.whites / 100) * weight * 255;
      r += val; g += val; b += val;
    }

    if (adj.blacks !== 0) {
      const weight = Math.pow(Math.max(0, 1 - lum), 4);
      const val = (adj.blacks / 100) * weight * 255;
      r += val; g += val; b += val;
    }

    // 5. Contrast
    r = contrastFactor * (r - 128) + 128;
    g = contrastFactor * (g - 128) + 128;
    b = contrastFactor * (b - 128) + 128;

    // Clamp
    r = Math.max(0, Math.min(255, r));
    g = Math.max(0, Math.min(255, g));
    b = Math.max(0, Math.min(255, b));

    // 6. Curves
    // Master Curve affects all channels identically
    r = lutMaster[Math.floor(r)];
    g = lutMaster[Math.floor(g)];
    b = lutMaster[Math.floor(b)];

    // Per-channel Curves
    r = lutRed[Math.floor(r)];
    g = lutGreen[Math.floor(g)];
    b = lutBlue[Math.floor(b)];

    // Saturation / Vibrance / HSL
    const needsHSL = adj.saturation !== 0 || adj.vibrance !== 0 || Object.values(adj.hsl).some(v => v.hue !== 0 || v.saturation !== 0 || v.luminance !== 0);

    if (needsHSL) {
      let [h, s, l] = rgbToHsl(r, g, b);

      s += (adj.saturation / 100);
      if (adj.vibrance !== 0) {
        s += (adj.vibrance / 100) * (1 - s);
      }

      const hueDeg = h * 360;
      let channel = '';
      if (hueDeg >= 330 || hueDeg < 15) channel = 'red';
      else if (hueDeg >= 15 && hueDeg < 45) channel = 'orange';
      else if (hueDeg >= 45 && hueDeg < 75) channel = 'yellow';
      else if (hueDeg >= 75 && hueDeg < 155) channel = 'green';
      else if (hueDeg >= 155 && hueDeg < 185) channel = 'aqua';
      else if (hueDeg >= 185 && hueDeg < 255) channel = 'blue';
      else if (hueDeg >= 255 && hueDeg < 285) channel = 'purple';
      else channel = 'magenta';

      const ch = adj.hsl[channel];
      if (ch) {
        h += (ch.hue / 360);
        s += (ch.saturation / 100);
        l += (ch.luminance / 100);
      }

      s = Math.max(0, Math.min(1, s));
      l = Math.max(0, Math.min(1, l));
      while (h < 0) h += 1;
      while (h > 1) h -= 1;

      const rgb = hslToRgb(h, s, l);
      r = rgb[0]; g = rgb[1]; b = rgb[2];
    }

    // Vignette
    if (adj.vignette !== 0) {
      const idx = i / 4;
      const x = idx % width;
      const y = Math.floor(idx / width);
      const cx = width / 2;
      const cy = height / 2;
      const maxDist = Math.sqrt(cx * cx + cy * cy);
      const dist = Math.sqrt((x - cx) ** 2 + (y - cy) ** 2);
      const percent = dist / maxDist;
      const vFactor = 1 - (percent * (adj.vignette / 100));
      if (adj.vignette > 0) {
        r *= Math.max(0, vFactor);
        g *= Math.max(0, vFactor);
        b *= Math.max(0, vFactor);
      }
    }

    dst[i] = Math.max(0, Math.min(255, r));
    dst[i + 1] = Math.max(0, Math.min(255, g));
    dst[i + 2] = Math.max(0, Math.min(255, b));
    dst[i + 3] = src[i + 3];
  }

  // Grain
  if (adj.grain > 0) {
    for (let i = 0; i < dst.length; i += 4) {
      const noise = (Math.random() - 0.5) * (adj.grain * 2);
      dst[i] = Math.max(0, Math.min(255, dst[i] + noise));
      dst[i + 1] = Math.max(0, Math.min(255, dst[i + 1] + noise));
      dst[i + 2] = Math.max(0, Math.min(255, dst[i + 2] + noise));
    }
  }

  ctx.putImageData(outputImage, 0, 0);
};