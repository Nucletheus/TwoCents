'use client';

import { createContext, useContext, useEffect, useState, ReactNode } from 'react';

type Theme = 'light' | 'dark';

export type PaletteName = 'fresh' | 'warm' | 'trust' | 'monochrome';

interface PaletteColors {
  primary: string;
  secondary: string;
  accent: string;
}

export const PALETTES: Record<PaletteName, { name: string; colors: PaletteColors }> = {
  fresh: {
    name: 'Fresh & Modern',
    colors: { primary: '#3b82f6', secondary: '#14b8a6', accent: '#3b82f6' } // blue-500, teal-500
  },
  warm: {
    name: 'Warm & Relational',
    colors: { primary: '#f43f5e', secondary: '#f59e0b', accent: '#f43f5e' } // rose-500, amber-500
  },
  trust: {
    name: 'Trust & Growth',
    colors: { primary: '#4f46e5', secondary: '#10b981', accent: '#4f46e5' } // indigo-600, emerald-500
  },
  monochrome: {
    name: 'Sleek Monochrome',
    colors: { primary: '#1e293b', secondary: '#94a3b8', accent: '#1e293b' } // slate-800, slate-400
  }
};

interface ThemeContextType {
  theme: Theme;
  palette: PaletteName;
  toggleTheme: () => void;
  setTheme: (theme: Theme) => void;
  setPalette: (palette: PaletteName) => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<Theme>('light');
  const [palette, setPaletteState] = useState<PaletteName>('fresh');
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    // Check localStorage or system preference
    const savedTheme = localStorage.getItem('theme') as Theme | null;
    const savedPalette = localStorage.getItem('palette') as PaletteName | null;
    
    const systemTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light';
      
    const initialTheme = savedTheme || systemTheme;
    const initialPalette = savedPalette && PALETTES[savedPalette] ? savedPalette : 'fresh';
    
    setThemeState(initialTheme);
    setPaletteState(initialPalette);
    
    applyTheme(initialTheme);
    applyPalette(initialPalette);
  }, []);

  const applyTheme = (newTheme: Theme) => {
    const root = document.documentElement;
    if (newTheme === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  };
  
  const applyPalette = (newPalette: PaletteName) => {
    const root = document.documentElement;
    const colors = PALETTES[newPalette].colors;
    
    root.style.setProperty('--palette-primary', colors.primary);
    root.style.setProperty('--palette-secondary', colors.secondary);
    root.style.setProperty('--palette-accent', colors.accent);
    
    // Also update the accent color variable used by Tailwind
    root.style.setProperty('--accent', colors.accent);
  };

  const setTheme = (newTheme: Theme) => {
    setThemeState(newTheme);
    localStorage.setItem('theme', newTheme);
    applyTheme(newTheme);
  };
  
  const setPalette = (newPalette: PaletteName) => {
    setPaletteState(newPalette);
    localStorage.setItem('palette', newPalette);
    applyPalette(newPalette);
  };

  const toggleTheme = () => {
    const newTheme = theme === 'light' ? 'dark' : 'light';
    setTheme(newTheme);
  };

  // Always provide the context, but only apply theme class after mount to prevent hydration mismatch
  return (
    <ThemeContext.Provider value={{ theme, palette, toggleTheme, setTheme, setPalette }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}
