'use client';

import { X, Moon, Sun, Check } from 'lucide-react';
import { useTheme, PALETTES, PaletteName } from './ThemeProvider';
import { useState, useEffect } from 'react';

interface SettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function SettingsDialog({ isOpen, onClose }: SettingsDialogProps) {
  const { theme, toggleTheme, palette, setPalette } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!isOpen || !mounted) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
      <div className="bg-background w-full max-w-md rounded-xl shadow-2xl border border-border overflow-hidden">
        <div className="flex items-center justify-between p-4 border-b border-border">
          <h2 className="text-lg font-semibold">Settings</h2>
          <button
            onClick={onClose}
            className="p-1 rounded-md hover:bg-muted transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="p-6 space-y-8">
          {/* Appearance Section */}
          <section>
            <h3 className="text-sm font-medium text-muted-foreground mb-4 uppercase tracking-wider">Appearance</h3>
            
            {/* Theme Toggle */}
            <div className="flex items-center justify-between mb-6">
              <span className="text-sm font-medium">Dark Mode</span>
              <button
                onClick={toggleTheme}
                className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ${
                  theme === 'dark' ? 'bg-palette-primary' : 'bg-input bg-gray-200'
                }`}
                style={{ backgroundColor: theme === 'dark' ? 'var(--palette-primary)' : '' }}
              >
                <span
                  className={`${
                    theme === 'dark' ? 'translate-x-6' : 'translate-x-1'
                  } inline-block h-4 w-4 transform rounded-full bg-white transition-transform`}
                />
              </button>
            </div>

            {/* Palette Selector */}
            <div className="space-y-3">
              <span className="text-sm font-medium block">Color Palette</span>
              <div className="grid grid-cols-2 gap-3">
                {(Object.entries(PALETTES) as [PaletteName, typeof PALETTES[PaletteName]][]).map(([key, value]) => (
                  <button
                    key={key}
                    onClick={() => setPalette(key)}
                    className={`relative flex items-center p-2 rounded-lg border-2 transition-all ${
                      palette === key 
                        ? 'border-palette-primary bg-accent/5' 
                        : 'border-transparent hover:bg-muted'
                    }`}
                    style={{ borderColor: palette === key ? 'var(--palette-primary)' : 'transparent' }}
                  >
                    <div className="flex -space-x-1 mr-3">
                      <div 
                        className="w-4 h-4 rounded-full border border-background" 
                        style={{ backgroundColor: value.colors.primary }}
                      />
                      <div 
                        className="w-4 h-4 rounded-full border border-background" 
                        style={{ backgroundColor: value.colors.secondary }}
                      />
                    </div>
                    <span className="text-sm font-medium">{value.name.split(' ')[0]}</span>
                    {palette === key && (
                      <div className="absolute right-2 top-1/2 -translate-y-1/2 text-palette-primary" style={{ color: 'var(--palette-primary)' }}>
                        <Check className="w-4 h-4" />
                      </div>
                    )}
                  </button>
                ))}
              </div>
            </div>
          </section>
        </div>
        
        <div className="p-4 bg-muted/30 border-t border-border flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium rounded-md hover:bg-muted transition-colors"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

