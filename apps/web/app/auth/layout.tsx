'use client';

import { useEffect } from 'react';

export default function AuthLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  useEffect(() => {
    // Force light mode on auth pages by removing dark class
    const html = document.documentElement;
    html.classList.remove('dark');
    
    // Store original theme to restore later
    const originalTheme = html.classList.contains('dark') ? 'dark' : 'light';
    
    return () => {
      // Optionally restore theme when leaving auth pages
      // For now, we'll let the ThemeProvider handle it
    };
  }, []);

  return <>{children}</>;
}

