'use client';

import { useEffect, useMemo, useState, useRef, useCallback } from 'react';
import Sidebar from './Sidebar';
import { MainScrollProvider } from './MainScrollProvider';
import PageHeader from './PageHeader';
import Select from './ui/Select';
import QuickActionBar from './QuickActionBar';
import { useHousehold } from './HouseholdProvider';
import { Menu } from 'lucide-react';
import Header from './Header';

type MainLayoutPageHeader = {
  title: string;
  subtitle?: string;
  secondary?: React.ReactNode;
  secondarySlotRef?: (el: HTMLDivElement | null) => void;
  showHouseholdSelect?: boolean;
};

export default function MainLayout({
  children,
  pageHeader,
  contentVariant,
  contentPaddingY = 'default',
}: {
  children: React.ReactNode;
  pageHeader?: MainLayoutPageHeader;
  contentVariant?: 'default' | 'full-width';
  contentPaddingY?: 'default' | 'compact' | 'none';
}) {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [isMobile, setIsMobile] = useState(false);
  const [isMobileSidebarOpen, setIsMobileSidebarOpen] = useState(false);
  const openSidebarRef = useRef<(() => void) | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLDivElement>(null);
  const footerRef = useRef<HTMLDivElement>(null);
  const [headerHeight, setHeaderHeight] = useState(0);
  const [footerHeight, setFooterHeight] = useState(0);
  const { households, selectedHouseholdId, setSelectedHouseholdId, loading: householdsLoading } =
    useHousehold();

  useEffect(() => {
    const checkSidebarState = () => {
      const savedState = localStorage.getItem('sidebar-collapsed');
      if (savedState !== null) {
        setSidebarCollapsed(savedState === 'true');
      }
    };
    checkSidebarState();
    
    const checkMobile = () => {
      setIsMobile(window.innerWidth < 768);
    };
    checkMobile();
    window.addEventListener('resize', checkMobile);
    
    // Listen for sidebar state changes from Sidebar component
    const handleSidebarStateChange = () => checkSidebarState();
    window.addEventListener('sidebar-state-changed', handleSidebarStateChange);
    
    return () => {
      window.removeEventListener('sidebar-state-changed', handleSidebarStateChange);
      window.removeEventListener('resize', checkMobile);
    };
  }, []);

  const handleMenuClick = useCallback(() => {
    // Call the exposed function to open sidebar
    if (openSidebarRef.current) {
      openSidebarRef.current();
    }
  }, []);

  const handleOpenRef = useCallback((openFn: () => void) => {
    openSidebarRef.current = openFn;
  }, []);

  const handleMobileStateChange = useCallback((isOpen: boolean) => {
    setIsMobileSidebarOpen(isOpen);
  }, []);

  const paddingY =
    contentPaddingY === 'none' ? 'py-0' : contentPaddingY === 'compact' ? 'py-2' : 'py-1';

  const householdSelect = useMemo(() => {
    if (!pageHeader || pageHeader.showHouseholdSelect === false) return null;
    if (householdsLoading || households.length === 0) return null;

    return (
      <Select
        value={selectedHouseholdId || ''}
        onChange={(e) => setSelectedHouseholdId(e.target.value)}
        className="h-10 w-44"
      >
        {households.map((h) => (
          <option key={h.id} value={h.id}>
            {h.name}
          </option>
        ))}
      </Select>
    );
  }, [
    households,
    householdsLoading,
    pageHeader,
    selectedHouseholdId,
    setSelectedHouseholdId,
  ]);

  const leading = useMemo(() => {
    if (!isMobile || isMobileSidebarOpen) return undefined;

    return (
      <button
        onClick={handleMenuClick}
        className="inline-flex h-9 w-9 items-center justify-center rounded-notion border border-border/60 bg-card/50 backdrop-blur hover:bg-hover transition-colors"
        aria-label="Open sidebar"
      >
        <Menu className="h-5 w-5" />
      </button>
    );
  }, [handleMenuClick, isMobile, isMobileSidebarOpen]);

  useEffect(() => {
    const headerEl = headerRef.current;
    const footerEl = footerRef.current;
    if (!headerEl && !footerEl) return;

    // Initial measure
    if (headerEl) setHeaderHeight(Math.ceil(headerEl.getBoundingClientRect().height));
    if (footerEl) setFooterHeight(Math.ceil(footerEl.getBoundingClientRect().height));

    if (typeof ResizeObserver === 'undefined') return;

    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const next = Math.ceil(entry.contentRect.height);
        if (entry.target === headerEl) setHeaderHeight(next);
        if (entry.target === footerEl) setFooterHeight(next);
      }
    });

    if (headerEl) ro.observe(headerEl);
    if (footerEl) ro.observe(footerEl);

    return () => ro.disconnect();
  }, []);

  return (
    <div className="flex h-screen overflow-hidden relative">
      <Sidebar onOpenRef={handleOpenRef} onMobileStateChange={handleMobileStateChange} />
      <main
        className="flex-1 bg-background transition-all relative z-0 overflow-hidden"
        style={{
          marginLeft: isMobile ? '0' : (sidebarCollapsed ? '64px' : '240px'),
        }}
      >
        <MainScrollProvider scrollRef={scrollRef}>
          <div
            ref={scrollRef}
            className="h-full overflow-y-auto"
            style={{ paddingTop: headerHeight, paddingBottom: footerHeight }}
          >
            {contentVariant === 'full-width' ? (
              <div className={`px-2 ${paddingY}`}>
                {children}
              </div>
            ) : (
              <div className="mx-auto max-w-7xl px-2.5 py-2.5">
                {children}
              </div>
            )}
          </div>
        </MainScrollProvider>

        {/* Header overlay */}
        <div ref={headerRef} className="absolute top-0 left-0 right-0 z-[60]">
          {!pageHeader && <Header onMenuClick={handleMenuClick} isSidebarOpen={isMobileSidebarOpen} />}
          {pageHeader && (
            <PageHeader
              title={pageHeader.title}
              subtitle={pageHeader.subtitle}
              leading={leading}
              householdSelect={householdSelect}
              secondary={pageHeader.secondary}
              secondarySlotRef={pageHeader.secondarySlotRef}
            />
          )}
        </div>

        {/* Footer overlay */}
        <div ref={footerRef} className="absolute bottom-0 left-0 right-0 z-[60]">
          <QuickActionBar />
        </div>
      </main>
    </div>
  );
}

