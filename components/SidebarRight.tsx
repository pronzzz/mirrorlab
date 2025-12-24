import React, { useState } from 'react';
import { useEditor } from '../context/EditorContext';
import { HSL_CHANNELS } from '../constants';
import Panel from './ui/Panel';
import Slider from './ui/Slider';
import ToneCurve from './panels/ToneCurve';
import { ChevronDown, ChevronRight } from 'lucide-react';

const SidebarRight: React.FC = () => {
  const { state, dispatch } = useEditor();
  const { adjustments } = state;
  const [openHSLChannel, setOpenHSLChannel] = useState<string | null>(null);

  const update = (key: keyof typeof adjustments, val: number) => {
    dispatch({ type: 'UPDATE_ADJUSTMENT', payload: { [key]: val } });
  };

  return (
    <aside className="w-80 h-full glass-panel border-l border-white/10 flex flex-col z-20 overflow-y-auto">
      <div className="p-4 border-b border-white/10 flex justify-between items-center bg-black/20 sticky top-0 z-10 backdrop-blur-md">
        <h2 className="text-sm font-semibold tracking-wide text-white">Adjustments</h2>
        <span className="text-xs px-2 py-0.5 rounded bg-blue-600/30 text-blue-300 border border-blue-500/30">Edit</span>
      </div>

      <div className="flex-1 pb-20">
        {/* Light */}
        <Panel title="Light" defaultOpen={true}>
          <Slider label="Exposure" value={adjustments.exposure} min={-4} max={4} step={0.1} onChange={(v) => update('exposure', v)} onReset={() => update('exposure', 0)} />
          <Slider label="Contrast" value={adjustments.contrast} min={-100} max={100} onChange={(v) => update('contrast', v)} onReset={() => update('contrast', 0)} />
          <Slider label="Highlights" value={adjustments.highlights} min={-100} max={100} onChange={(v) => update('highlights', v)} onReset={() => update('highlights', 0)} />
          <Slider label="Shadows" value={adjustments.shadows} min={-100} max={100} onChange={(v) => update('shadows', v)} onReset={() => update('shadows', 0)} />
          <Slider label="Whites" value={adjustments.whites} min={-100} max={100} onChange={(v) => update('whites', v)} onReset={() => update('whites', 0)} />
          <Slider label="Blacks" value={adjustments.blacks} min={-100} max={100} onChange={(v) => update('blacks', v)} onReset={() => update('blacks', 0)} />
        </Panel>

        {/* Color */}
        <Panel title="Color">
          <div className="mb-4">
            <Slider label="Temp" value={adjustments.temperature} min={-100} max={100} onChange={(v) => update('temperature', v)} onReset={() => update('temperature', 0)} />
            <Slider label="Tint" value={adjustments.tint} min={-100} max={100} onChange={(v) => update('tint', v)} onReset={() => update('tint', 0)} />
          </div>
          <div className="border-t border-white/5 pt-4">
            <Slider label="Vibrance" value={adjustments.vibrance} min={-100} max={100} onChange={(v) => update('vibrance', v)} onReset={() => update('vibrance', 0)} />
            <Slider label="Saturation" value={adjustments.saturation} min={-100} max={100} onChange={(v) => update('saturation', v)} onReset={() => update('saturation', 0)} />
          </div>
        </Panel>

        {/* Tone Curve */}
        <Panel title="Tone Curve">
          <div className="p-2">
            <ToneCurve />
          </div>
        </Panel>

        {/* HSL */}
        <Panel title="HSL / Color Mixer">
          <div className="space-y-1">
            {HSL_CHANNELS.map(channel => (
              <div key={channel.id} className="border border-white/5 rounded-lg overflow-hidden">
                <button 
                  onClick={() => setOpenHSLChannel(openHSLChannel === channel.id ? null : channel.id)}
                  className="w-full flex items-center justify-between p-2 hover:bg-white/5 transition-colors"
                >
                  <div className="flex items-center gap-2">
                    <div className="w-3 h-3 rounded-full" style={{ backgroundColor: channel.color }}></div>
                    <span className="text-xs font-medium text-gray-300 capitalize">{channel.label}</span>
                  </div>
                  {openHSLChannel === channel.id ? <ChevronDown size={14} className="text-gray-500" /> : <ChevronRight size={14} className="text-gray-500" />}
                </button>
                
                {openHSLChannel === channel.id && (
                  <div className="p-3 bg-black/20 space-y-3 animate-in fade-in slide-in-from-top-1 duration-200">
                     <Slider 
                       label="Hue" 
                       value={adjustments.hsl[channel.id].hue} 
                       min={-100} max={100} 
                       onChange={(v) => dispatch({type: 'UPDATE_HSL', payload: { channel: channel.id, params: { hue: v } } })} 
                     />
                     <Slider 
                       label="Saturation" 
                       value={adjustments.hsl[channel.id].saturation} 
                       min={-100} max={100} 
                       onChange={(v) => dispatch({type: 'UPDATE_HSL', payload: { channel: channel.id, params: { saturation: v } } })} 
                     />
                     <Slider 
                       label="Luminance" 
                       value={adjustments.hsl[channel.id].luminance} 
                       min={-100} max={100} 
                       onChange={(v) => dispatch({type: 'UPDATE_HSL', payload: { channel: channel.id, params: { luminance: v } } })} 
                     />
                  </div>
                )}
              </div>
            ))}
          </div>
        </Panel>

        {/* Effects */}
        <Panel title="Effects">
           <Slider label="Clarity" value={adjustments.clarity} min={-100} max={100} onChange={(v) => update('clarity', v)} onReset={() => update('clarity', 0)} />
           <Slider label="Vignette" value={adjustments.vignette} min={0} max={100} onChange={(v) => update('vignette', v)} onReset={() => update('vignette', 0)} />
           <Slider label="Grain" value={adjustments.grain} min={0} max={100} onChange={(v) => update('grain', v)} onReset={() => update('grain', 0)} />
        </Panel>
      </div>
    </aside>
  );
};

export default SidebarRight;