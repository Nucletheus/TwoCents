'use client';

import React, { createContext, useContext } from 'react';

type MainScrollContextValue = {
  scrollRef: React.RefObject<HTMLDivElement>;
};

const MainScrollContext = createContext<MainScrollContextValue | undefined>(undefined);

export function MainScrollProvider({
  scrollRef,
  children,
}: {
  scrollRef: React.RefObject<HTMLDivElement>;
  children: React.ReactNode;
}) {
  return (
    <MainScrollContext.Provider value={{ scrollRef }}>
      {children}
    </MainScrollContext.Provider>
  );
}

export function useMainScroll() {
  const ctx = useContext(MainScrollContext);
  if (!ctx) {
    throw new Error('useMainScroll must be used within a MainScrollProvider');
  }
  return ctx;
}


