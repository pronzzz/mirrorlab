import React, { useRef, useState, useEffect } from 'react';

interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (val: number) => void;
  onReset?: () => void;
  unit?: string;
}

const Slider: React.FC<SliderProps> = ({ label, value, min, max, step = 1, onChange, onReset, unit = '' }) => {
  // Internal state for immediate feedback
  const [internalValue, setInternalValue] = useState(value);
  const trackRef = useRef<HTMLDivElement>(null);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Sync internal value when prop changes (if not dragging, or to force update)
  useEffect(() => {
    setInternalValue(value);
  }, [value]);

  const percentage = ((internalValue - min) / (max - min)) * 100;

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = Number(e.target.value);
    setInternalValue(newValue);

    // Debounce the heavy context update
    if (timeoutRef.current) clearTimeout(timeoutRef.current);

    // Use a small delay. 16ms is one frame. 
    // If we want it to feel responsive but not choke blocking main thread, 
    // we can try 5-10ms or just use requestAnimationFrame implicitly by waiting.
    // However, basic throttle is safer for "heavy" operations.
    timeoutRef.current = setTimeout(() => {
      onChange(newValue);
    }, 16);
  };

  const handleDoubleClick = () => {
    if (onReset) onReset();
  };

  return (
    <div className="mb-4 select-none group">
      <div className="flex justify-between items-center mb-2 text-xs text-gray-400 font-medium">
        <label className="group-hover:text-text transition-colors cursor-default font-bold tracking-wide uppercase text-[10px]">{label}</label>
        <span
          className="cursor-pointer hover:text-highlight font-mono tabular-nums text-accent"
          onDoubleClick={handleDoubleClick}
          title="Double click to reset"
        >
          {internalValue}{unit}
        </span>
      </div>
      <div className="relative w-full h-10 flex items-center">
        {/* Track background */}
        <div className="absolute w-full h-2 bg-background neumorphic-inset rounded-full overflow-hidden">
          {/* Fill */}
          <div
            className="h-full bg-accent transition-all duration-75 ease-out"
            style={{ width: `${percentage}%` }}
          />
        </div>
        {/* Native Range Input (Invisible but handles interaction) */}
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={internalValue}
          onChange={handleChange}
          onDoubleClick={handleDoubleClick}
          className="absolute w-full h-full opacity-0 cursor-pointer z-10"
        />
        {/* Custom Thumb (Visual only) */}
        <div
          className="absolute h-4 w-4 bg-background border border-white/10 rounded-full shadow-[0_2px_4px_rgba(0,0,0,0.3)] pointer-events-none transition-all duration-75 ease-out flex items-center justify-center"
          style={{ left: `calc(${percentage}% - 8px)` }}
        >
          <div className="w-1.5 h-1.5 rounded-full bg-highlight"></div>
        </div>
      </div>
    </div>
  );
};

export default Slider;