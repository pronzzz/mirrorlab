import React, { useState } from 'react';
import Panel from './ui/Panel';
import { useEditor } from '../context/EditorContext';
import { Plus, Trash2 } from 'lucide-react';
import { PRESETS } from '../constants';

const SidebarLeft: React.FC = () => {
  const { state, dispatch } = useEditor();
  const [newPresetName, setNewPresetName] = useState('');

  const handleSavePreset = () => {
    if (!newPresetName.trim()) return;
    const newPreset = {
      id: `custom-${Date.now()}`,
      name: newPresetName,
      adjustments: { ...state.adjustments },
    };
    dispatch({ type: 'SAVE_PRESET', payload: newPreset });
    setNewPresetName('');
  };

  const isDefaultPreset = (id: string) => PRESETS.some(p => p.id === id);

  return (
    <aside className="w-full md:w-64 h-full glass-panel border-r border-white/5 flex flex-col z-20 shadow-xl">
      <div className="p-4 border-b border-white/5 bg-background/50">
        <h2 className="text-sm font-bold tracking-widest text-transparent bg-clip-text bg-gradient-to-r from-highlight to-accent uppercase">Library</h2>
      </div>
      <div className="flex-1 overflow-y-auto p-2 scrollbar-hide">
        <Panel title="Presets">
          <div className="px-2 pb-4 border-b border-white/5 mb-4">
            <div className="flex gap-2">
              <input
                type="text"
                value={newPresetName}
                onChange={(e) => setNewPresetName(e.target.value)}
                placeholder="Preset name..."
                className="flex-1 bg-background neumorphic-inset px-3 py-2 text-xs text-text focus:outline-none focus:ring-1 focus:ring-accent rounded-lg placeholder:text-gray-600"
              />
              <button
                onClick={handleSavePreset}
                className="p-2 neumorphic-btn text-highlight hover:text-accent transition-colors rounded-lg flex-shrink-0"
                title="Save Current as Preset"
              >
                <Plus size={16} />
              </button>
            </div>
          </div>
          <div className="space-y-3">
            {state.presets.map((preset) => (
              <div
                key={preset.id}
                className="group flex items-center justify-between p-3 neumorphic hover:scale-[1.02] transition-transform cursor-pointer text-gray-400 hover:text-text border border-transparent hover:border-accent/10"
                onClick={() => dispatch({ type: 'APPLY_PRESET', payload: preset.adjustments })}
              >
                <span className="text-xs font-medium">{preset.name}</span>
                {!isDefaultPreset(preset.id) && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      dispatch({ type: 'DELETE_PRESET', payload: preset.id });
                    }}
                    className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-400 transition-all"
                  >
                    <Trash2 size={12} />
                  </button>
                )}
              </div>
            ))}
          </div>
        </Panel>
        <Panel title="History">
          <div className="space-y-2 max-h-64 overflow-y-auto px-1 pt-1">
            {state.history.map((_, idx) => (
              <div
                key={idx}
                className={`p-2 text-[10px] rounded-lg cursor-pointer font-medium transition-all ${idx === state.historyIndex
                  ? 'neumorphic-inset text-highlight border-l-2 border-highlight pl-2'
                  : 'text-gray-500 hover:text-gray-300 hover:bg-white/5'
                  }`}
              >
                {idx === 0 ? 'Original' : `Adjustment ${idx}`}
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </aside>
  );
};

export default SidebarLeft;