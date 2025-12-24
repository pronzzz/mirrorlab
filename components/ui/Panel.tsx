import React, { useState } from 'react';

interface PanelProps {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
}

const Panel: React.FC<PanelProps> = ({ title, children, defaultOpen = false }) => {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border-b border-neutral-800 last:border-0">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center justify-between w-full py-3 px-4 text-sm font-medium text-gray-300 hover:bg-white/5 transition-colors focus:outline-none"
      >
        <span>{title}</span>
        <svg
          className={`w-4 h-4 text-gray-500 transform transition-transform duration-200 ${isOpen ? 'rotate-180' : ''}`}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      <div 
        className={`overflow-hidden transition-all duration-300 ease-in-out ${isOpen ? 'max-h-[800px] opacity-100' : 'max-h-0 opacity-0'}`}
      >
        <div className="p-4 pt-0 text-sm">
          {children}
        </div>
      </div>
    </div>
  );
};

export default Panel;