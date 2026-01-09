# Release Notes - v1.2.0 (Style & Animation Overhaul)

## 🚀 Overview
This release brings a major visual and interactive overhaul to MirrorLab. Inspired by modern design trends like **Neumorphism**, **Coolors palettes**, and **Animista animations**, we've transformed the editor into a more tactile and premium-feeling workspace.

## ✨ Key Changes

### 🎨 Visual Identity & Colors
- **New Color Palette**: Transitioned from a standard dark mode to a sophisticated Charcoal & Purple theme.
  - Backgrounds: `#333333` / `#3d3d3d`
  - Accents: Vibrant Sage (`#86a59c`) and Light Purple (`#7d5ba6`)
  - Highlights: Soft Green (`#89ce94`) for active states.
- **Improved Typography**: Refined spacing and weights for better legibility on high-density displays.

### 🧊 Neumorphic UI Component Library
- **Tactile Controls**: All buttons now feature a "Soft UI" / Neumorphic effect with double-shadow highlights for a pressable, 3D feel.
- **Recessed Inputs**: Search bars and filename fields now use an "inset" shadow style, creating a natural depth in the layout.
- **Framed Canvas**: The main editing area is now nested in a neumorphic-inset container, separating the workspace from the tools.

### 🎭 Animation & Interactivity
- **Smooth Transitions**:
  - `scale-in`: Applied to the "No Image" state and modals for a dynamic entry.
  - `slide-up`: Sidebars and adjustment panels now slide smoothly into view.
  - `fade-in`: Subtle text and icon fades for a less jarring layout shift.
- **Enhanced Feedback**: Real-time hover effects on sliders and buttons provide immediate visual cues.

### 🛠 Component Refinement
- **Custom Sliders**: Range inputs have been completely restyled with custom thumbs and accent-colored tracks.
- **Redesigned Toolbar**: A more compact, glass-effect header with animated dropdowns for export options.
- **Styled Empty State**: A new centered dashboard view when no image is loaded, featuring animated icons and vibrant gradients.

## 📦 Technical Updates
- Updated Tailwind configuration with custom theme tokens.
- Optimized CSS utility classes for glassmorphism and backdrop filters.
- Implemented `requestAnimationFrame` for smoother canvas rendering during rapid adjustments.

---
*Reflect Your Vision with the all-new MirrorLab UI.*
