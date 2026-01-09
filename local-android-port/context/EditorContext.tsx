import React, { createContext, useContext, useReducer, ReactNode, useEffect } from 'react';
import { Adjustments, EditorState, Action, Preset } from '../types';
import { INITIAL_ADJUSTMENTS, PRESETS } from '../constants';

const initialState: EditorState = {
  imageSrc: null,
  filename: 'untitled',
  adjustments: INITIAL_ADJUSTMENTS,
  history: [INITIAL_ADJUSTMENTS],
  historyIndex: 0,
  zoom: 1,
  isProcessing: false,
  presets: PRESETS,
};

function reducer(state: EditorState, action: Action): EditorState {
  switch (action.type) {
    case 'SET_IMAGE':
      return {
        ...state,
        imageSrc: action.payload.src,
        filename: action.payload.filename,
        adjustments: INITIAL_ADJUSTMENTS,
        history: [INITIAL_ADJUSTMENTS],
        historyIndex: 0,
      };
    case 'UPDATE_ADJUSTMENT': {
      const newAdj = { ...state.adjustments, ...action.payload };
      const newHistory = state.history.slice(0, state.historyIndex + 1);
      newHistory.push(newAdj);
      return {
        ...state,
        adjustments: newAdj,
        history: newHistory,
        historyIndex: newHistory.length - 1,
      };
    }
    case 'UPDATE_HSL': {
      const newHsl = { ...state.adjustments.hsl, [action.payload.channel]: { ...state.adjustments.hsl[action.payload.channel], ...action.payload.params } };
      const newAdj = { ...state.adjustments, hsl: newHsl };
      const newHistory = state.history.slice(0, state.historyIndex + 1);
      newHistory.push(newAdj);
      return {
        ...state,
        adjustments: newAdj,
        history: newHistory,
        historyIndex: newHistory.length - 1,
      };
    }
    case 'UPDATE_COLOR_GRADING': {
      const newColorGrading = {
        ...state.adjustments.colorGrading,
        [action.payload.region]: {
          ...state.adjustments.colorGrading[action.payload.region],
          ...action.payload.params
        }
      };
      const newAdj = { ...state.adjustments, colorGrading: newColorGrading };
      const newHistory = state.history.slice(0, state.historyIndex + 1);
      newHistory.push(newAdj);
      return {
        ...state,
        adjustments: newAdj,
        history: newHistory,
        historyIndex: newHistory.length - 1,
      };
    }
    case 'UPDATE_CURVE': {
      let newCurve = state.adjustments.curve;

      if ('channel' in action.payload) {
        // Partial update for specific channel
        newCurve = {
          ...state.adjustments.curve,
          [action.payload.channel]: action.payload.points
        };
      } else {
        // Full curve replacement
        newCurve = action.payload;
      }

      const newAdj = { ...state.adjustments, curve: newCurve };
      const newHistory = state.history.slice(0, state.historyIndex + 1);
      newHistory.push(newAdj);
      return {
        ...state,
        adjustments: newAdj,
        history: newHistory,
        historyIndex: newHistory.length - 1,
      };
    }
    case 'APPLY_PRESET': {
      const newAdj = action.payload;
      const newHistory = state.history.slice(0, state.historyIndex + 1);
      newHistory.push(newAdj);
      return {
        ...state,
        adjustments: newAdj,
        history: newHistory,
        historyIndex: newHistory.length - 1,
      };
    }
    case 'SAVE_PRESET': {
      const newPresets = [...state.presets, action.payload];
      localStorage.setItem('mirrorlab_presets', JSON.stringify(newPresets.filter(p => !PRESETS.find(base => base.id === p.id))));
      return {
        ...state,
        presets: newPresets,
      };
    }
    case 'DELETE_PRESET': {
      const newPresets = state.presets.filter(p => p.id !== action.payload);
      localStorage.setItem('mirrorlab_presets', JSON.stringify(newPresets.filter(p => !PRESETS.find(base => base.id === p.id))));
      return {
        ...state,
        presets: newPresets,
      };
    }
    case 'UNDO':
      if (state.historyIndex > 0) {
        return {
          ...state,
          historyIndex: state.historyIndex - 1,
          adjustments: state.history[state.historyIndex - 1],
        };
      }
      return state;
    case 'REDO':
      if (state.historyIndex < state.history.length - 1) {
        return {
          ...state,
          historyIndex: state.historyIndex + 1,
          adjustments: state.history[state.historyIndex + 1],
        };
      }
      return state;
    case 'RESET':
      const resetAdj = INITIAL_ADJUSTMENTS;
      const resetHistory = state.history.slice(0, state.historyIndex + 1);
      resetHistory.push(resetAdj);
      return {
        ...state,
        adjustments: resetAdj,
        history: resetHistory,
        historyIndex: resetHistory.length - 1,
      };
    case 'SET_ZOOM':
      return { ...state, zoom: action.payload };
    default:
      return (state as any);
  }
}

const EditorContext = createContext<{
  state: EditorState;
  dispatch: React.Dispatch<Action>;
}>({ state: initialState, dispatch: () => null });

export const EditorProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    const saved = localStorage.getItem('mirrorlab_presets');
    if (saved) {
      try {
        const customPresets = JSON.parse(saved);
        customPresets.forEach((p: Preset) => {
          if (!state.presets.find(existing => existing.id === p.id)) {
            dispatch({ type: 'SAVE_PRESET', payload: p });
          }
        });
      } catch (e) {
        console.error('Failed to load presets', e);
      }
    }
  }, []);

  return (
    <EditorContext.Provider value={{ state, dispatch }}>
      {children}
    </EditorContext.Provider>
  );
};

export const useEditor = () => useContext(EditorContext);