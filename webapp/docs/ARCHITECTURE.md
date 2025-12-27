# MirrorLab Architecture & Development Guide

This document provides a technical overview of MirrorLab for developers who want to understand the codebase and contribute.

## Architecture Overview

MirrorLab is a client-side Single Page Application (SPA) built with:
- **React**: UI components and state management.
- **Context API (`EditorContext`)**: Global state for image data, adjustments, and history (undo/redo).
- **HTML5 Canvas**: The core rendering engine. All image processing happens on a `<canvas>` element via pixel manipulation.

### Directory Structure

```
mirrorlab/
├── src/
│   ├── components/         # React UI Components
│   │   ├── panels/         # Editing panels (ToneCurve, HSL, etc.)
│   │   ├── MainCanvas.tsx  # The main image viewport
│   │   └── Toolbar.tsx     # Top bar (Import/Export)
│   ├── context/            # React Context (State Store)
│   ├── services/           # Core Logic
│   │   └── imageProcessor.ts # The Image Engine (Pixel pipeline)
│   ├── types.ts            # TypeScript Definitions
│   └── constants.ts        # Default values and configuration
```

## Core Systems

### 1. State Management (`EditorContext`)
The application state is monolithic and managed in `EditorContext.tsx`. Everything from the current zoom level to the complex Tone Curve state is stored here. Actions are dispatched to a reducer to update this state.

**Key State Object:**
```typescript
interface EditorState {
  imageSrc: string | null;      // Base64 or Blob URL of source
  adjustments: Adjustments;     // Current edit parameters
  history: Adjustments[];       // Undo stack
  // ...
}
```

### 2. The Image Pipeline (`imageProcessor.ts`)
This is the heart of the application. The `processImage` function takes the source `ImageData` and the current `Adjustments` object and writes the result to the destination canvas.

**Pipeline Order:**
1.  **Exposure**: Linear multiplication.
2.  **White Balance**: Channel scaling.
3.  **Clarity/Structure**: Local contrast enhancement (simulated).
4.  **Tone Mapping**: Highlights, Shadows, Whites, Blacks algorithms.
5.  **Contrast**: Sigmoid-like curve.
6.  **Tone Curve**:
    - Generates 4 Look-Up Tables (LUTs) using Spline Interpolation: Master, Red, Green, Blue.
    - Applies Master LUT to all channels.
    - Applies R, G, B LUTs to respective channels.
7.  **HSL / Color Graidng**: Converts RGB -> HSL, modifies channels, converts back to RGB.
8.  **Vignette & Grain**: Final effects.

### 3. RAW File Handling
Browsers cannot natively render RAW files (CR2, NEF, etc.).
- **Current Solution**: We use `exifr` to extract the *embedded JPEG preview* from the RAW file.
- **Why?**: It's instant and requires no massive WASM libraries. For 99% of web use cases, the embedded preview (often full resolution) is sufficient.
- **The Code**: Located in `Toolbar.tsx`. It detects RAW extensions and calls `exifr.thumbnailUrl()`.

### 4. Tone Curve Logic
- **Data Structure**: `CurveState` object containing 4 arrays of `Point` `{x, y}`.
- **Interpolation**: We use a Cubic Spline algorithm to generate a smooth 256-value LUT from the user's control points.
- **Interaction**: The `ToneCurve` component handles the SVG interaction logic (drag, add point) and dispatches updates to the Context.

## Building & Deploying

- **Dev**: `npm run dev` starts the Vite server.
- **Build**: `npm run build` generates static files in `dist/`.
- **Deployment**: The `dist/` folder can be hosted on any static site host (Vercel, Netlify, GitHub Pages).

## Future Technical Debt / Improvements
- **Web Workers**: Move `imageProcessor` to a Web Worker to prevent UI blocking during heavy edits.
- **WebGL**: Port pixel manipulation to WebGL shaders for GPU acceleration (critical for mobile/high-res).
- **WASM RAW Decoding**: Use `libraw` via WASM to allow true demosaicing and white balance adjustment of RAW data, rather than just using previews.
