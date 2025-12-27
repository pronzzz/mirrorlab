# Contributing to MirrorLab 📸

First off, thank you for considering contributing to MirrorLab! It's people like you who make open-source tools great.

## 🔍 How Can I Contribute?

### Reporting Bugs
- Use the GitHub [Issues](https://github.com/pronzzz/mirrorlab/issues) tab.
- Include your **browser version**, **OS**, and a **sample RAW file** if the bug is related to image decoding.

### Suggesting Enhancements
- Check the [ROADMAP.md](./ROADMAP.md) first to see if your idea is already planned.
- Open an issue labeled `enhancement` to discuss the feature before building it.

### Pull Requests
1. Fork the repo and create your branch from `main`.
2. If you've added code that should be tested, add tests!
3. Ensure the project builds locally: `npm run build`.
4. Update the documentation if you've changed the UI or workflow.

## 🛠️ Technical Guidelines

### Performance is Key
MirrorLab processes high-resolution images in the browser. When contributing:
- **Avoid unnecessary re-renders:** Use `React.memo` or `useMemo` for heavy UI components like the Curve Editor.
- **Canvas Operations:** Keep heavy loops out of the main thread where possible. We are looking to move core math to Web Workers or WASM.

### Coding Style
- **TypeScript:** We use strict mode. Avoid `any`.
- **Styling:** Use TailwindCSS for all UI components.
- **Icons:** Use Lucide-React.

## 🧪 Development Setup
```bash
# Install dependencies
npm install

# Run in development mode
npm run dev
