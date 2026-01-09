import React from 'react';
import { EditorProvider } from './context/EditorContext';
import Toolbar from './components/Toolbar';
import SidebarLeft from './components/SidebarLeft';
import SidebarRight from './components/SidebarRight';
import MainCanvas from './components/MainCanvas';

const AppContent = () => {
  const [activeTab, setActiveTab] = React.useState<'editor' | 'presets' | 'adjust'>('editor');

  return (
    <div className="flex flex-col h-screen w-screen bg-background text-text overflow-hidden font-sans">
      <Toolbar />
      <div className="flex flex-col md:flex-row flex-1 overflow-hidden relative p-4 gap-4">
        {/* On mobile, only show SidebarLeft if activeTab is 'presets' */}
        {/* Changed from inset-0 to bottom-sheet style layout on mobile */}
        <div
          className={`${activeTab === 'presets' ? 'flex' : 'hidden'} md:flex h-[50vh] md:h-auto w-full md:w-80 shrink-0 z-20 absolute md:static bottom-0 left-0 right-0 md:bg-transparent transition-transform duration-300 ease-in-out animate-slide-up`}
        >
          <SidebarLeft />
        </div>

        <MainCanvas />

        {/* On mobile, only show SidebarRight if activeTab is 'adjust' */}
        {/* Changed from inset-0 to bottom-sheet style layout on mobile */}
        <div
          className={`${activeTab === 'adjust' ? 'flex' : 'hidden'} md:flex h-[50vh] md:h-auto w-full md:w-80 shrink-0 z-20 absolute md:static bottom-0 left-0 right-0 md:bg-transparent justify-end transition-transform duration-300 ease-in-out animate-slide-up`}
        >
          <SidebarRight />
        </div>
      </div>

      {/* Mobile Tab Bar */}
      <div className="md:hidden h-16 glass-panel flex items-center justify-around z-30 shrink-0 mb-safe pb-2">
        <button
          onClick={() => setActiveTab('presets')}
          className={`p-3 rounded-xl flex flex-col items-center gap-1 transition-colors ${activeTab === 'presets' ? 'text-highlight bg-white/5' : 'text-gray-400'}`}
        >
          <span className="text-[10px] uppercase font-bold tracking-wider">Presets</span>
        </button>
        <button
          onClick={() => setActiveTab('editor')}
          className={`p-3 rounded-xl flex flex-col items-center gap-1 transition-colors ${activeTab === 'editor' ? 'text-highlight bg-white/5' : 'text-gray-400'}`}
        >
          <span className="text-[10px] uppercase font-bold tracking-wider">Editor</span>
        </button>
        <button
          onClick={() => setActiveTab('adjust')}
          className={`p-3 rounded-xl flex flex-col items-center gap-1 transition-colors ${activeTab === 'adjust' ? 'text-highlight bg-white/5' : 'text-gray-400'}`}
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