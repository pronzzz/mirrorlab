import React from 'react';
import { EditorProvider } from './context/EditorContext';
import Toolbar from './components/Toolbar';
import SidebarLeft from './components/SidebarLeft';
import SidebarRight from './components/SidebarRight';
import MainCanvas from './components/MainCanvas';

const AppContent = () => {
  return (
    <div className="flex flex-col h-screen w-screen bg-black text-gray-100 overflow-hidden font-sans">
      <Toolbar />
      <div className="flex flex-1 overflow-hidden relative">
        <SidebarLeft />
        <MainCanvas />
        <SidebarRight />
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