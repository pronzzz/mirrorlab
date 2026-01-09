import React, { useRef, useState } from 'react';
import { useEditor } from '../context/EditorContext';
import { Download, ChevronDown, FileType, FileText, Image as ImageIcon } from 'lucide-react';
import exifr from 'exifr';

const Toolbar: React.FC = () => {
  const { state, dispatch } = useEditor();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [showExportOptions, setShowExportOptions] = useState(false);
  const [exportName, setExportName] = useState('');
  const [exportFormat, setExportFormat] = useState<'image/png' | 'image/jpeg' | 'image/webp'>('image/png');
  const [quality, setQuality] = useState(0.9);

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      try {
        // Check for RAW formats
        const extension = file.name.split('.').pop()?.toLowerCase();
        const isRaw = ['dng', 'cr2', 'nef', 'arw', 'orf', 'rw2', 'raf', 'tif', 'tiff'].includes(extension || '');

        if (isRaw) {
          try {
            // Attempt to extract embedded preview
            const thumbUrl = await exifr.thumbnailUrl(file);
            if (thumbUrl) {
              dispatch({
                type: 'SET_IMAGE',
                payload: {
                  src: thumbUrl,
                  filename: file.name
                }
              });
              const nameWithoutExt = file.name.split('.').slice(0, -1).join('.');
              setExportName(`${nameWithoutExt}-edited`);
              return;
            } else {
              console.warn("No embedded thumbnail found in RAW file, attempting fallback.");
            }
          } catch (err) {
            console.warn("Failed to extract preview from RAW:", err);
          }
        }

        const reader = new FileReader();
        reader.onload = (event) => {
          if (event.target?.result) {
            dispatch({
              type: 'SET_IMAGE',
              payload: {
                src: event.target.result as string,
                filename: file.name
              }
            });
            const nameWithoutExt = file.name.split('.').slice(0, -1).join('.');
            setExportName(`${nameWithoutExt}-edited`);
          }
        };
        reader.readAsDataURL(file);
      } catch (error) {
        console.error("Error loading file:", error);
      }
    }
  };

  const handleExport = () => {
    const canvas = document.querySelector('canvas');
    if (canvas) {
      const link = document.createElement('a');
      const ext = exportFormat.split('/')[1];
      link.download = `${exportName || 'edited-image'}.${ext}`;
      link.href = canvas.toDataURL(exportFormat, quality);
      link.click();
      setShowExportOptions(false);
    }
  };

  return (
    <header className="h-16 flex items-center justify-between px-6 z-30 relative glass-panel border-b border-white/5 shadow-lg">
      <div className="flex items-center gap-6">
        <div className="font-bold text-xl tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-highlight to-accent animate-fade-in">
          MirrorLab
        </div>
        <div className="h-8 w-px bg-surface mx-2 neumorphic-inset"></div>
        <button
          onClick={() => fileInputRef.current?.click()}
          className="neumorphic-btn px-4 py-2 text-sm font-medium text-gray-400 hover:text-highlight flex items-center gap-2 group"
        >
          <FileType size={18} className="group-hover:text-accent transition-colors" />
          <span className="hidden md:inline">Import</span>
        </button>
        <input
          type="file"
          ref={fileInputRef}
          className="hidden"
          accept="image/*,.dng,.tiff,.tif,.cr2,.nef,.arw,.orf,.rw2,.raf"
          onChange={handleFileChange}
        />
      </div>

      <div className="flex items-center gap-4">
        <div className="flex bg-surface rounded-xl p-1.5 neumorphic-inset items-center gap-1">
          <button
            onClick={() => dispatch({ type: 'UNDO' })}
            disabled={state.historyIndex <= 0}
            className="p-2 hover:text-highlight rounded-lg disabled:opacity-30 transition-colors disabled:cursor-not-allowed"
            title="Undo"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" /></svg>
          </button>
          <button
            onClick={() => dispatch({ type: 'REDO' })}
            disabled={state.historyIndex >= state.history.length - 1}
            className="p-2 hover:text-highlight rounded-lg disabled:opacity-30 transition-colors disabled:cursor-not-allowed"
            title="Redo"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" /></svg>
          </button>
        </div>

        <div className="relative">
          <button
            onClick={() => setShowExportOptions(!showExportOptions)}
            className={`neumorphic-btn px-5 py-2 text-sm font-bold flex items-center gap-2 ${showExportOptions ? 'text-highlight shadow-inner' : 'text-primary'}`}
          >
            <Download size={18} />
            <span className="hidden md:inline">Export</span>
            <ChevronDown size={14} className={`transition-transform duration-300 ${showExportOptions ? 'rotate-180' : ''}`} />
          </button>

          {showExportOptions && (
            <div className="absolute top-full right-0 mt-4 w-80 bg-surface neumorphic border border-white/5 p-5 z-50 animate-scale-in origin-top-right">
              <div className="space-y-5">
                <div>
                  <label className="block text-[10px] uppercase tracking-widest text-gray-500 mb-2 font-bold ml-1">Filename</label>
                  <input
                    type="text"
                    value={exportName}
                    onChange={(e) => setExportName(e.target.value)}
                    placeholder="Enter filename..."
                    className="w-full bg-background neumorphic-inset px-4 py-3 text-sm text-text focus:outline-none focus:ring-1 focus:ring-accent/50 transition-all rounded-xl placeholder:text-gray-600"
                  />
                </div>

                <div>
                  <label className="block text-[10px] uppercase tracking-widest text-gray-500 mb-2 font-bold ml-1">Format</label>
                  <div className="grid grid-cols-3 gap-3">
                    {(['image/png', 'image/jpeg', 'image/webp'] as const).map((format) => (
                      <button
                        key={format}
                        onClick={() => setExportFormat(format)}
                        className={`py-2 rounded-lg text-xs font-bold transition-all duration-200 ${exportFormat === format
                          ? 'neumorphic-btn active text-highlight border-accent/20'
                          : 'neumorphic-btn text-gray-500 hover:text-gray-300'
                          }`}
                      >
                        {format.split('/')[1].toUpperCase()}
                      </button>
                    ))}
                  </div>
                </div>

                {(exportFormat === 'image/jpeg' || exportFormat === 'image/webp') && (
                  <div className="animate-fade-in">
                    <div className="flex justify-between mb-2 ml-1">
                      <label className="text-[10px] uppercase tracking-widest text-gray-500 font-bold">Quality</label>
                      <span className="text-xs text-highlight font-mono font-bold">{Math.round(quality * 100)}%</span>
                    </div>
                    <input
                      type="range"
                      min="0.1"
                      max="1.0"
                      step="0.05"
                      value={quality}
                      onChange={(e) => setQuality(parseFloat(e.target.value))}
                      className="w-full h-2 bg-background rounded-lg appearance-none cursor-pointer accent-accent neumorphic-inset p-[2px]"
                    />
                  </div>
                )}

                <button
                  onClick={handleExport}
                  className="w-full neumorphic-btn py-3 text-sm font-bold text-highlight hover:text-accent transition-all flex items-center justify-center gap-2 mt-2 group"
                >
                  <Download size={18} className="group-hover:scale-110 transition-transform" />
                  Download Image
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </header >
  );
};

export default Toolbar;