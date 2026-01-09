import React, { useRef, useEffect, useState } from 'react';
import { useEditor } from '../context/EditorContext';
import { processImage } from '../services/imageProcessor';

const MainCanvas: React.FC = () => {
  const { state } = useEditor();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Stores the original image data to apply filters on top of
  const [originalImageData, setOriginalImageData] = useState<ImageData | null>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });

  // Load Image
  useEffect(() => {
    if (!state.imageSrc) return;

    const img = new Image();
    img.crossOrigin = "Anonymous";
    img.src = state.imageSrc;
    img.onload = () => {
      // Logic to fit image within max dimensions while maintaining aspect ratio
      // For performance, we might downscale really large images for preview
      const MAX_PREVIEW_SIZE = 2000;
      let width = img.width;
      let height = img.height;

      if (width > MAX_PREVIEW_SIZE || height > MAX_PREVIEW_SIZE) {
        const ratio = width / height;
        if (width > height) {
          width = MAX_PREVIEW_SIZE;
          height = width / ratio;
        } else {
          height = MAX_PREVIEW_SIZE;
          width = height * ratio;
        }
      }

      setDimensions({ width, height });

      // Create an offscreen canvas to extract ImageData
      const osc = document.createElement('canvas');
      osc.width = width;
      osc.height = height;
      const ctx = osc.getContext('2d');
      if (ctx) {
        ctx.drawImage(img, 0, 0, width, height);
        const data = ctx.getImageData(0, 0, width, height);
        setOriginalImageData(data);
      }
    };
  }, [state.imageSrc]);

  // Apply Effects
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !originalImageData) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Run processing
    // Use requestAnimationFrame to avoid blocking UI on rapid slider moves
    let animationId: number;

    const render = () => {
      processImage(ctx, originalImageData, state.adjustments);
    };

    animationId = requestAnimationFrame(render);

    return () => cancelAnimationFrame(animationId);
  }, [originalImageData, state.adjustments]);

  // Measure container for "contain" logic
  const [containerSize, setContainerSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    if (!containerRef.current) return;

    // Initial measure
    const measure = () => {
      if (containerRef.current) {
        setContainerSize({
          width: containerRef.current.clientWidth,
          height: containerRef.current.clientHeight
        });
      }
    };
    measure();

    const observer = new ResizeObserver(measure);
    observer.observe(containerRef.current);

    return () => observer.disconnect();
  }, []);

  // Calculate display size (object-fit: contain logic)
  const getDisplayDimensions = () => {
    if (dimensions.width === 0 || dimensions.height === 0 || containerSize.width === 0) {
      return { width: 'auto', height: 'auto' }; // Fallback
    }

    const PADDING = 0.9; // 90%
    const maxWidth = containerSize.width * PADDING;
    const maxHeight = containerSize.height * PADDING;

    const imgRatio = dimensions.width / dimensions.height;
    const containerRatio = maxWidth / maxHeight;

    let targetWidth, targetHeight;

    if (imgRatio > containerRatio) {
      // Image is wider than container (relative to height)
      targetWidth = maxWidth;
      targetHeight = maxWidth / imgRatio;
    } else {
      // Image is taller
      targetHeight = maxHeight;
      targetWidth = targetHeight * imgRatio;
    }

    return { width: `${targetWidth}px`, height: `${targetHeight}px` };
  };

  const displayStyle = getDisplayDimensions();

  // Handle Zoom/Pan CSS transform (Simplification for demo)
  const scale = state.zoom;

  if (!state.imageSrc) {
    return (
      <div className="flex-1 flex items-center justify-center bg-background m-2 rounded-2xl neumorphic-inset border border-white/5">
        <div className="text-center p-8 rounded-2xl neumorphic border border-white/5 animate-scale-in">
          <div className="mb-4 text-accent/50 flex justify-center">
            <svg className="w-16 h-16" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
            </svg>
          </div>
          <p className="mb-2 text-xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-highlight to-accent uppercase tracking-widest">No Image Loaded</p>
          <p className="text-sm text-gray-400 font-medium">Drag and drop or click Import to start</p>
        </div>
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className="flex-1 relative overflow-hidden flex items-center justify-center bg-background neumorphic-inset shadow-inner m-2 rounded-2xl border border-white/5"
    >
      <div
        className="relative shadow-2xl transition-transform duration-200 ease-out rounded-lg overflow-hidden border border-white/10"
        style={{
          width: displayStyle.width,
          height: displayStyle.height,
          transform: `scale(${scale})`,
        }}
      >
        <canvas
          ref={canvasRef}
          width={dimensions.width}
          height={dimensions.height}
          className="w-full h-full object-contain"
        />
      </div>
    </div>
  );
};

export default MainCanvas;