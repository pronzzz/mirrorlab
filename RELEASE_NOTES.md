# Release Notes

## v1.1.0 (2025-12-27)

### New Features
- **Mobile Browser UI Compatibility**: Complete overhaul of the interface to support mobile devices with a responsive layout, touch-friendly navigation, and adaptive sidebars.
- **Color Grading (Split Toning)**: New tool in the "Adjustments" panel allowing independent tint control for Shadows and Highlights. This allows for cinematic color grading effects. User experience improved with auto-saturation adjustment.

### Improvements
- **Responsive Navigation**: Added a bottom tab bar for mobile devices to easily switch between Editor, Presets, and Adjustments.
- **Performance**: Optimized rendering pipeline to ensure changes are applied efficiently without draining battery on mobile devices.
- **UI Polish**: Updated sidebar visibility logic to be more robust across different screen sizes.
- **RAW Import**: Restored embedded preview support to ensure RAW files from various cameras can be opened and verified (displays embedded JPEG).

### Bug Fixes
- Fixed potential layout issues where sidebars could overlay the canvas on small screens.
- Addressed issue where Color Grading hue changes were not visible if saturation was zero.
- Fixed issue with RAW images importing as small thumbnails.
