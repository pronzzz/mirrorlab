import React from 'react';
import { EditorProvider } from './context/EditorContext';
import Toolbar from './components/Toolbar';
import SidebarLeft from './components/SidebarLeft';
import SidebarRight from './components/SidebarRight';
import MainCanvas from './components/MainCanvas';

const AppContent = () => {
  const [activeTab, setActiveTab] = React.useState<'editor' | 'presets' | 'adjust'>('editor');

  return (
    <div className="flex flex-col h-screen w-screen bg-black text-gray-100 overflow-hidden font-sans">
      <Toolbar />
      <div className="flex flex-col md:flex-row flex-1 overflow-hidden relative">
        {/* On mobile, only show SidebarLeft if activeTab is 'presets' */}
        <div className={`${activeTab === 'presets' ? 'flex' : 'hidden'} md:flex h-full md:h-auto w-full md:w-auto z-20 absolute md:static inset-0 bg-black/90 md:bg-transparent`}>
          <SidebarLeft />
        </div>

        <MainCanvas />

        {/* On mobile, only show SidebarRight if activeTab is 'adjust' */}
        <div className={`${activeTab === 'adjust' ? 'flex' : 'hidden'} md:flex h-full md:h-auto w-full md:w-auto z-20 absolute md:static inset-0 bg-black/90 md:bg-transparent justify-end`}>
          <SidebarRight />
        </div>
      </div>

      {/* Mobile Tab Bar */}
      <div className="md:hidden h-14 glass-panel border-t border-white/10 flex items-center justify-around z-30 shrink-0">
        <button
          onClick={() => setActiveTab('presets')}
          className={`p-2 flex flex-col items-center gap-1 ${activeTab === 'presets' ? 'text-blue-400' : 'text-gray-400'}`}
        >
          <span className="text-[10px] uppercase font-bold tracking-wider">Presets</span>
        </button>
        <button
          onClick={() => setActiveTab('editor')}
          className={`p-2 flex flex-col items-center gap-1 ${activeTab === 'editor' ? 'text-blue-400' : 'text-gray-400'}`}
        >
          <span className="text-[10px] uppercase font-bold tracking-wider">Editor</span>
        </button>
        <button
          onClick={() => setActiveTab('adjust')}
          className={`p-2 flex flex-col items-center gap-1 ${activeTab === 'adjust' ? 'text-blue-400' : 'text-gray-400'}`}
        >
          <span className="text-[10px] uppercase font-bold tracking-wider">Adjust</span>
        </button>
      </div>
    </div>
  );
};

const App: React.FC = () => {
  return (
    <EditorProvider>
      <AppContent />
    </EditorProvider>
  );
};

export default App;