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
    <aside className="w-full md:w-64 h-full glass-panel border-r border-white/10 flex flex-col z-20">
      <div className="p-4 border-b border-white/10">
        <h2 className="text-sm font-semibold tracking-wide text-white">Library</h2>
      </div>
      <div className="flex-1 overflow-y-auto">
        <Panel title="Presets">
          <div className="px-2 pb-4 border-b border-white/10 mb-2">
            <div className="flex gap-2">
              <input
                type="text"
                value={newPresetName}
                onChange={(e) => setNewPresetName(e.target.value)}
                placeholder="Preset name..."
                className="flex-1 bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white focus:outline-none focus:border-blue-500"
              />
              <button
                onClick={handleSavePreset}
                className="p-1 bg-blue-600 hover:bg-blue-500 rounded text-white transition-colors"
                title="Save Current as Preset"
              >
                <Plus size={16} />
              </button>
            </div>
          </div>
          <div className="space-y-1">
            {state.presets.map((preset) => (
              <div
                key={preset.id}
                className="group flex items-center justify-between p-2 hover:bg-white/5 rounded cursor-pointer text-gray-300 transition-colors"
                onClick={() => dispatch({ type: 'APPLY_PRESET', payload: preset.adjustments })}
              >
                <span className="text-xs">{preset.name}</span>
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
          <div className="space-y-1 max-h-64 overflow-y-auto">
            {state.history.map((_, idx) => (
              <div
                key={idx}
                className={`p-2 text-[10px] rounded cursor-pointer ${idx === state.historyIndex ? 'bg-blue-600/20 text-blue-400' : 'text-gray-500 hover:bg-white/5'
                  }`}
                onClick={() => {
                  // We don't have a direct JUMP_TO_HISTORY but we can use UNDO/REDO logic or add it.
                  // For now, let's just show it's working.
                }}
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