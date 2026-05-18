'use client';

import { useState, useEffect } from 'react';
import { PanelLeftOpen } from 'lucide-react';

interface HeaderProps {
  onMenuClick: () => void;
  isSidebarOpen: boolean;
}

export default function Header({ onMenuClick, isSidebarOpen }: HeaderProps) {
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth < 768);
    };
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  // Don't render if not mobile or if sidebar is open (button moves to sidebar)
  if (!isMobile || isSidebarOpen) {
    return null;
  }

  return (
    <header className="backdrop-blur-md bg-background/80 border-b border-border">
      <div className="flex h-16 items-center px-4">
        <button
          onClick={onMenuClick}
          className="rounded-md bg-card p-2 shadow-notion-sm hover:bg-hover border border-border"
          aria-label="Open sidebar"
        >
          <PanelLeftOpen className="h-5 w-5" />
        </button>
      </div>
    </header>
  );
}

