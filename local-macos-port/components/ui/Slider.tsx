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
  const [isDragging, setIsDragging] = useState(false);
  const trackRef = useRef<HTMLDivElement>(null);

  const percentage = ((value - min) / (max - min)) * 100;

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onChange(Number(e.target.value));
  };

  const handleDoubleClick = () => {
    if (onReset) onReset();
  };

  return (
    <div className="mb-4 select-none group">
      <div className="flex justify-between items-center mb-1.5 text-xs text-gray-400 font-medium">
        <label className="group-hover:text-gray-200 transition-colors cursor-default">{label}</label>
        <span 
          className="cursor-pointer hover:text-blue-400 font-mono tabular-nums"
          onDoubleClick={handleDoubleClick}
          title="Double click to reset"
        >
          {value}{unit}
        </span>
      </div>
      <div className="relative w-full h-5 flex items-center">
        {/* Track background */}
        <div className="absolute w-full h-1 bg-neutral-700 rounded-full overflow-hidden">
          {/* Fill */}
          <div 
            className="h-full bg-blue-500 transition-all duration-75 ease-out"
            style={{ width: `${percentage}%` }}
          />
        </div>
        {/* Native Range Input (Invisible but handles interaction) */}
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={handleChange}
          onDoubleClick={handleDoubleClick}
          className="absolute w-full h-full opacity-0 cursor-pointer z-10"
        />
        {/* Custom Thumb (Visual only) */}
        <div 
          className="absolute h-3 w-3 bg-white rounded-full shadow-md pointer-events-none transition-all duration-75 ease-out"
          style={{ left: `calc(${percentage}% - 6px)` }}
        />
      </div>
    </div>
  );
};

export default Slider;