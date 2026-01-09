import React, { useRef, useState, useEffect } from 'react';
import { useEditor } from '../../context/EditorContext';
import { CurveState, Point } from '../../types';
import { DEFAULT_CURVE } from '../../constants';

const ToneCurve: React.FC = () => {
  const { state, dispatch } = useEditor();
  const { curve } = state.adjustments;
  const svgRef = useRef<SVGSVGElement>(null);

  const [activeChannel, setActiveChannel] = useState<keyof CurveState>('master');
  const [activePointIndex, setActivePointIndex] = useState<number | null>(null);

  // Ensure curve state exists for current structure (during migration/dev)
  const currentPoints = curve[activeChannel] || DEFAULT_CURVE[activeChannel];

  // Helper to get color based on channel
  const getChannelColor = (channel: keyof CurveState) => {
    switch (channel) {
      case 'red': return '#ef4444';
      case 'green': return '#22c55e';
      case 'blue': return '#3b82f6';
      default: return '#ffffff'; // Master
    }
  };

  const getCoordinates = (e: React.MouseEvent | MouseEvent) => {
    if (!svgRef.current) return null;
    const rect = svgRef.current.getBoundingClientRect();
    const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const y = Math.max(0, Math.min(1, 1 - (e.clientY - rect.top) / rect.height));
    return { x, y };
  };

  const updateCurve = (newPoints: Point[]) => {
    // Keep points sorted by X
    newPoints.sort((a, b) => a.x - b.x);
    dispatch({
      type: 'UPDATE_CURVE',
      payload: { channel: activeChannel, points: newPoints }
    });
  };

  const handleMouseDown = (e: React.MouseEvent, index: number) => {
    e.stopPropagation();
    setActivePointIndex(index);
  };

  const handleSvgMouseDown = (e: React.MouseEvent) => {
    const coords = getCoordinates(e);
    if (!coords) return;

    // Check if close to existing point to select it instead of adding
    const threshold = 0.05;
    const existingIdx = currentPoints.findIndex(p => Math.abs(p.x - coords.x) < threshold && Math.abs(p.y - coords.y) < threshold);

    if (existingIdx !== -1) {
      setActivePointIndex(existingIdx);
      return;
    }

    // Add new point
    const newPoints = [...currentPoints, coords];
    updateCurve(newPoints);
    // Find where it was inserted to set active index
    // Since we sort, we need to find the point that matches our coords
    // But float precision might be tricky. Let's just rely on the fact that sort is deterministic.
    // Ideally we would want to know the index after sort.
    // For now, let's just wait for next render or effect, but effect is tricky with multiple renders.
    // We can just rely on the user clicking the point if they want to drag immediately, 
    // OR we could manually find the index of the newly added point after sorting.
    // Let's do nothing for active index on *creation* for simplicity unless we refactor sort.
    // Actually, dragging immediately after creation is standard.
    // Let's defer "activePointIndex" setting until mouse move if it was creation logic, 
    // but here we just added it. Let's not auto-select for drag to keep it simple and robust.
    // User can just click and drag.
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (activePointIndex === null) return;
    const coords = getCoordinates(e);
    if (!coords) return;

    const newPoints = [...currentPoints];
    const p = newPoints[activePointIndex];

    // Constrain endpoints
    // First point must be at x=0? Usually yes, but technically Black point can move on X?
    // Standard Tone Curve:
    // Black Point (originally 0,0) can move along X=0 axis (Output Black) or Y=0 axis (Input Black).
    // Usually standard tools pin the X of endpoints or allow them to move freely but clamps to edges.
    // For this implementation, let's assume standard behavior:
    // Leftmost point is constrained to x=0? No, usually you can slide the black point to the right (crushing blacks).
    // But keeping existing logic:
    // If it is the first point in the array...
    // Actually sorting makes "first point" dynamic.
    // Let's just update the point coordinates and let the sort handle the order.
    // BUT we must prevent points from crossing each other on X axis?
    // Standard curves often enforce X monotonicity by sorting.

    // Removing index-based constraints (endpoints) allows full flexibility (e.g. inverting curve).
    // The only hard constraint is we likely want one point at X=0 and one at X=1 eventually,
    // or the curve is undefined outside the range of points.
    // Interpolator usually handles extension.

    newPoints[activePointIndex] = coords;
    updateCurve(newPoints);
  };

  const handleMouseUp = () => {
    setActivePointIndex(null);
  };

  // Handle Numeric Input Changes
  const handlePointInputChange = (axis: 'x' | 'y', value: string) => {
    if (activePointIndex === null) return;
    const num = parseFloat(value);
    if (isNaN(num)) return;

    // Value given is 0-255, convert to 0-1
    const normalized = Math.max(0, Math.min(1, num / 255));

    const newPoints = [...currentPoints];
    newPoints[activePointIndex] = {
      ...newPoints[activePointIndex],
      [axis]: normalized
    };
    updateCurve(newPoints);
  };

  useEffect(() => {
    if (activePointIndex !== null) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
    }
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [activePointIndex, currentPoints]);

  const pointsStr = currentPoints.map(p => `${p.x * 100},${(1 - p.y) * 100}`).join(' ');
  const activeColor = getChannelColor(activeChannel);

  const activePoint = activePointIndex !== null ? currentPoints[activePointIndex] : null;

  return (
    <div className="flex flex-col gap-4">

      {/* Channel Selectors */}
      <div className="flex items-center justify-between px-2">
        <span className="text-xs font-medium text-gray-400 uppercase tracking-wider">Channel</span>
        <div className="flex gap-2">
          {(['master', 'red', 'green', 'blue'] as const).map(ch => (
            <button
              key={ch}
              onClick={() => { setActiveChannel(ch); setActivePointIndex(null); }}
              className={`w-6 h-6 rounded-full border-2 transition-all ${activeChannel === ch
                  ? 'border-white scale-110 shadow-lg'
                  : 'border-transparent opacity-50 hover:opacity-100 hover:scale-105'
                }`}
              style={{ backgroundColor: getChannelColor(ch) }}
              title={ch.charAt(0).toUpperCase() + ch.slice(1)}
            />
          ))}
        </div>
      </div>

      <div className="relative w-full aspect-square bg-neutral-800 rounded border border-neutral-700 select-none overflow-hidden cursor-crosshair group">
        {/* Grid */}
        <div className="absolute inset-0 grid grid-cols-4 grid-rows-4 pointer-events-none opacity-20">
          {[...Array(16)].map((_, i) => <div key={i} className="border border-gray-500"></div>)}
        </div>

        <svg
          ref={svgRef}
          className="w-full h-full"
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          onMouseDown={handleSvgMouseDown}
        >
          <line x1="0" y1="100" x2="100" y2="0" stroke="#444" strokeDasharray="4" strokeWidth="0.5" />

          <polyline
            points={pointsStr}
            fill="none"
            stroke={activeColor}
            strokeWidth="2"
            vectorEffect="non-scaling-stroke"
            className="transition-all duration-150 ease-out"
          />

          {currentPoints.map((p, i) => (
            <circle
              key={i}
              cx={p.x * 100}
              cy={(1 - p.y) * 100}
              r={activePointIndex === i ? 6 : 4}
              fill={activePointIndex === i ? '#fff' : activeColor}
              stroke="black"
              strokeWidth="1"
              className="cursor-pointer transition-all duration-100 ease-out hover:r-6"
              onMouseDown={(e) => handleMouseDown(e, i)}
            />
          ))}
        </svg>

        <div className="absolute top-2 right-2 flex gap-2">
          <button
            className="text-[10px] bg-black/50 hover:bg-black/70 px-2 py-1 rounded text-white backdrop-blur border border-white/10"
            onClick={(e) => {
              e.stopPropagation();
              dispatch({
                type: 'UPDATE_CURVE',
                payload: { channel: activeChannel, points: [...DEFAULT_CURVE[activeChannel]] }
              });
            }}
          >
            Reset {activeChannel === 'master' ? '' : activeChannel}
          </button>
        </div>
      </div>

      {/* Numeric Inputs */}
      {activePoint && (
        <div className="grid grid-cols-2 gap-2 animate-in fade-in slide-in-from-top-1 px-2">
          <div className="bg-white/5 rounded p-2 border border-white/5">
            <label className="text-[10px] uppercase text-gray-500 block mb-1">Input (X)</label>
            <input
              type="number"
              min="0" max="255"
              value={Math.round(activePoint.x * 255)}
              onChange={(e) => handlePointInputChange('x', e.target.value)}
              className="w-full bg-transparent text-sm font-mono border-b border-white/20 focus:border-blue-500 outline-none text-white text-right"
            />
          </div>
          <div className="bg-white/5 rounded p-2 border border-white/5">
            <label className="text-[10px] uppercase text-gray-500 block mb-1">Output (Y)</label>
            <input
              type="number"
              min="0" max="255"
              value={Math.round(activePoint.y * 255)}
              onChange={(e) => handlePointInputChange('y', e.target.value)}
              className="w-full bg-transparent text-sm font-mono border-b border-white/20 focus:border-blue-500 outline-none text-white text-right"
            />
          </div>
        </div>
      )}
    </div>
  );
};

export default ToneCurve;