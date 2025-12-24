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

  // Handle Zoom/Pan CSS transform (Simplification for demo)
  const scale = state.zoom;

  if (!state.imageSrc) {
    return (
      <div className="flex-1 flex items-center justify-center bg-black/50 text-neutral-500">
        <div className="text-center">
          <p className="mb-2 text-lg">No image loaded</p>
          <p className="text-sm opacity-50">Drag and drop or click Import</p>
        </div>
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className="flex-1 relative overflow-hidden flex items-center justify-center bg-[#050505] shadow-inner"
    >
      <div
        className="relative shadow-2xl transition-transform duration-75 ease-out"
        style={{
          aspectRatio: `${dimensions.width} / ${dimensions.height}`,
          maxWidth: '90%',
          maxHeight: '90%',
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