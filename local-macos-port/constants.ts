import { Adjustments, Point, CurveState } from './types';

const LINEAR_CURVE: Point[] = [
  { x: 0, y: 0 },
  { x: 0.25, y: 0.25 },
  { x: 0.5, y: 0.5 },
  { x: 0.75, y: 0.75 },
  { x: 1, y: 1 }
];

export const DEFAULT_CURVE: CurveState = {
  master: [...LINEAR_CURVE],
  red: [...LINEAR_CURVE],
  green: [...LINEAR_CURVE],
  blue: [...LINEAR_CURVE]
};

export const INITIAL_ADJUSTMENTS: Adjustments = {
  exposure: 0,
  contrast: 0,
  highlights: 0,
  shadows: 0,
  whites: 0,
  blacks: 0,
  temperature: 0,
  tint: 0,
  vibrance: 0,
  saturation: 0,
  clarity: 0,
  vignette: 0,
  grain: 0,
  hsl: {
    red: { hue: 0, saturation: 0, luminance: 0 },
    orange: { hue: 0, saturation: 0, luminance: 0 },
    yellow: { hue: 0, saturation: 0, luminance: 0 },
    green: { hue: 0, saturation: 0, luminance: 0 },
    aqua: { hue: 0, saturation: 0, luminance: 0 },
    blue: { hue: 0, saturation: 0, luminance: 0 },
    purple: { hue: 0, saturation: 0, luminance: 0 },
    magenta: { hue: 0, saturation: 0, luminance: 0 },
  },
  colorGrading: {
    shadows: { hue: 0, saturation: 0 },
    highlights: { hue: 0, saturation: 0 }
  },
  curve: DEFAULT_CURVE,
};

export const HSL_CHANNELS = [
  { id: 'red', label: 'Red', color: '#ef4444' },
  { id: 'orange', label: 'Orange', color: '#f97316' },
  { id: 'yellow', label: 'Yellow', color: '#eab308' },
  { id: 'green', label: 'Green', color: '#22c55e' },
  { id: 'aqua', label: 'Aqua', color: '#06b6d4' },
  { id: 'blue', label: 'Blue', color: '#3b82f6' },
  { id: 'purple', label: 'Purple', color: '#a855f7' },
  { id: 'magenta', label: 'Magenta', color: '#ec4899' },
];

export const PRESETS = [
  {
    id: 'natural',
    name: 'Natural',
    adjustments: { ...INITIAL_ADJUSTMENTS, contrast: 10, saturation: 5 }
  },
  {
    id: 'vivid',
    name: 'Vivid',
    adjustments: { ...INITIAL_ADJUSTMENTS, exposure: 0.2, contrast: 20, saturation: 30, vibrance: 10 }
  },
  {
    id: 'bw-high-contrast',
    name: 'B&W High Contrast',
    adjustments: { ...INITIAL_ADJUSTMENTS, saturation: -100, contrast: 40, exposure: 0.1 }
  },
  {
    id: 'matte',
    name: 'Matte',
    adjustments: {
      ...INITIAL_ADJUSTMENTS, contrast: -20, blacks: 20, curve: {
        ...DEFAULT_CURVE,
        master: [
          { x: 0, y: 0.1 },
          { x: 0.25, y: 0.25 },
          { x: 0.5, y: 0.5 },
          { x: 0.75, y: 0.75 },
          { x: 1, y: 0.9 }
        ]
      }
    }
  },
  {
    id: 'vintage-warm',
    name: 'Vintage Warm',
    adjustments: { ...INITIAL_ADJUSTMENTS, temperature: 20, tint: 5, contrast: 10, vignette: 20 }
  },
  {
    id: 'cool-shadows',
    name: 'Cool Shadows',
    adjustments: {
      ...INITIAL_ADJUSTMENTS, temperature: -15, hsl: {
        ...INITIAL_ADJUSTMENTS.hsl,
        blue: { hue: 0, saturation: 10, luminance: -10 }
      }
    }
  }
];

