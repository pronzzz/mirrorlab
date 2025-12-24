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
    <header className="h-14 glass-panel border-b border-white/10 flex items-center justify-between px-4 z-30 relative">
      <div className="flex items-center gap-4">
        <div className="font-bold text-lg tracking-tight bg-gradient-to-r from-blue-400 to-blue-200 bg-clip-text text-transparent">
          MirrorLab
        </div>
        <div className="h-6 w-px bg-white/10 mx-2"></div>
        <button
          onClick={() => fileInputRef.current?.click()}
          className="text-sm font-medium hover:text-white text-gray-400 transition-colors flex items-center gap-2"
        >
          <FileType size={16} />
          Import
        </button>
        <input
          type="file"
          ref={fileInputRef}
          className="hidden"
          accept="image/*,.dng,.tiff,.tif,.cr2,.nef,.arw,.orf,.rw2,.raf"
          onChange={handleFileChange}
        />
      </div>

      <div className="flex items-center gap-2">
        <div className="flex bg-black/30 rounded-lg p-1 border border-white/5">
          <button
            onClick={() => dispatch({ type: 'UNDO' })}
            disabled={state.historyIndex <= 0}
            className="p-1.5 hover:bg-white/10 rounded disabled:opacity-30 transition-colors"
            title="Undo"
          >
            <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" /></svg>
          </button>
          <button
            onClick={() => dispatch({ type: 'REDO' })}
            disabled={state.historyIndex >= state.history.length - 1}
            className="p-1.5 hover:bg-white/10 rounded disabled:opacity-30 transition-colors"
            title="Redo"
          >
            <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" /></svg>
          </button>
        </div>
        <div className="h-6 w-px bg-white/10 mx-2"></div>

        <div className="relative">
          <button
            onClick={() => setShowExportOptions(!showExportOptions)}
            className="bg-blue-600 hover:bg-blue-500 text-white text-sm px-4 py-1.5 rounded-md font-medium shadow-lg shadow-blue-500/20 transition-all border border-blue-400/20 flex items-center gap-2"
          >
            <Download size={16} />
            Export
            <ChevronDown size={14} className={`transition-transform ${showExportOptions ? 'rotate-180' : ''}`} />
          </button>

          {showExportOptions && (
            <div className="absolute top-full right-0 mt-2 w-72 bg-gray-900/95 backdrop-blur-xl border border-white/10 rounded-xl shadow-2xl p-4 z-50 animate-in fade-in zoom-in duration-200">
              <div className="space-y-4">
                <div>
                  <label className="block text-[10px] uppercase tracking-wider text-gray-500 mb-1.5 font-semibold">Filename</label>
                  <input
                    type="text"
                    value={exportName}
                    onChange={(e) => setExportName(e.target.value)}
                    placeholder="Enter filename..."
                    className="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500 transition-colors"
                  />
                </div>

                <div>
                  <label className="block text-[10px] uppercase tracking-wider text-gray-500 mb-1.5 font-semibold">Format</label>
                  <div className="grid grid-cols-3 gap-2">
                    {(['image/png', 'image/jpeg', 'image/webp'] as const).map((format) => (
                      <button
                        key={format}
                        onClick={() => setExportFormat(format)}
                        className={`py-1.5 rounded-lg text-xs font-medium border transition-all ${exportFormat === format
                          ? 'bg-blue-600 border-blue-500 text-white'
                          : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10'
                          }`}
                      >
                        {format.split('/')[1].toUpperCase()}
                      </button>
                    ))}
                  </div>
                </div>

                {(exportFormat === 'image/jpeg' || exportFormat === 'image/webp') && (
                  <div>
                    <div className="flex justify-between mb-1.5">
                      <label className="text-[10px] uppercase tracking-wider text-gray-500 font-semibold">Quality</label>
                      <span className="text-[10px] text-blue-400 font-medium">{Math.round(quality * 100)}%</span>
                    </div>
                    <input
                      type="range"
                      min="0.1"
                      max="1.0"
                      step="0.05"
                      value={quality}
                      onChange={(e) => setQuality(parseFloat(e.target.value))}
                      className="w-full h-1.5 bg-white/10 rounded-lg appearance-none cursor-pointer accent-blue-500"
                    />
                  </div>
                )}

                <button
                  onClick={handleExport}
                  className="w-full bg-blue-600 hover:bg-blue-500 text-white text-sm py-2.5 rounded-lg font-bold shadow-lg shadow-blue-500/20 transition-all flex items-center justify-center gap-2 mt-2"
                >
                  <Download size={16} />
                  Download
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </header>
  );
};

export default Toolbar;