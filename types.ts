export interface Point {
  x: number;
  y: number;
}

export interface CurveState {
  master: Point[];
  red: Point[];
  green: Point[];
  blue: Point[];
}

export interface HSLChannel {
  hue: number;
  saturation: number;
  luminance: number;
}

export interface HSLParams {
  red: HSLChannel;
  orange: HSLChannel;
  yellow: HSLChannel;
  green: HSLChannel;
  aqua: HSLChannel;
  blue: HSLChannel;
  purple: HSLChannel;
  magenta: HSLChannel;
}

export interface Adjustments {
  exposure: number;
  contrast: number;
  highlights: number;
  shadows: number;
  whites: number;
  blacks: number;
  temperature: number;
  tint: number;
  vibrance: number;
  saturation: number;
  clarity: number;
  vignette: number;
  grain: number;
  hsl: HSLParams;
  curve: CurveState;
}

export interface Preset {
  id: string;
  name: string;
  adjustments: Adjustments;
}

export interface EditorState {
  imageSrc: string | null;
  adjustments: Adjustments;
  history: Adjustments[];
  historyIndex: number;
  zoom: number;
  isProcessing: boolean;
  filename: string;
  presets: Preset[];
}

export type Action =
  | { type: 'SET_IMAGE'; payload: { src: string; filename: string } }
  | { type: 'UPDATE_ADJUSTMENT'; payload: Partial<Adjustments> }
  | { type: 'UPDATE_HSL'; payload: { channel: keyof HSLParams; params: Partial<HSLChannel> } }
  | { type: 'UPDATE_CURVE'; payload: { channel: keyof CurveState; points: Point[] } | CurveState }
  | { type: 'APPLY_PRESET'; payload: Adjustments }
  | { type: 'SAVE_PRESET'; payload: Preset }
  | { type: 'DELETE_PRESET'; payload: string }
  | { type: 'UNDO' }
  | { type: 'REDO' }
  | { type: 'RESET' }
  | { type: 'SET_ZOOM'; payload: number };

export enum PanelTab {
  LIGHT = 'Light',
  COLOR = 'Color',
  EFFECTS = 'Effects',
  DETAIL = 'Detail'
}

