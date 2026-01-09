# 🗺️ MirrorLab Roadmap

This document outlines the planned features and milestones for MirrorLab. As an open-source project, these goals are subject to change based on community feedback and contributions.

## 🟢 Phase 1: Core & Stability (Current Focus)
- [ ] **Performance Optimization:** Transition from heavy Canvas 2D operations to specialized WebGL shaders for real-time responsiveness.
- [ ] **History & Undo/Redo:** Implement a robust state management system to track edit history.
- [ ] **Unit Testing:** Add Vitest/Jest coverage for the custom spline interpolation and color math engines.
- [ ] **Expanded RAW Support:** Integration of a full `libraw` WASM build to improve decoding quality.

## 🟡 Phase 2: Advanced Editing Tools
- [ ] **Selective Masking:** Linear gradients, radial gradients, and "brush" masks for localized adjustments.
- [ ] **AI Subject Selection:** Integration of lightweight ML models (like MediaPipe) for one-click background or subject masking.
- [ ] **Lens Corrections:** Profiles for common lenses to fix distortion and chromatic aberration.
- [ ] **Healing & Cloning:** Tools to remove spots and unwanted objects.

## 🟠 Phase 3: Platform Expansion
- [ ] **Desktop Apps:** Wrap MirrorLab in **Tauri** or **Electron** for native macOS, Windows, and Linux builds.
- [ ] **Mobile Preview:** Responsive UI overhaul specifically for tablet and mobile browser workflows.
- [ ] **File System Access API:** Allow users to "Open Folder" directly from their local drive without manual imports.

## 🔵 Phase 4: Ecosystem & Workflow
- [ ] **Preset System:** Export and import `.mlpreset` files to share editing styles.
- [ ] **Batch Processing:** Apply edits to multiple photos simultaneously.
- [ ] **Cloud Interop:** Optional integration with Google Drive/Dropbox for saving projects.

---
*Want to see a feature? [Open an issue](https://github.com/pronzzz/mirrorlab/issues).*
