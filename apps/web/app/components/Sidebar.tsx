'use client';

import { useState, useEffect } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import { isStandaloneApp } from '@/lib/standalone';
import Link from 'next/link';
import Logo from './Logo';
import { useTheme } from './ThemeProvider';
import {
  PanelLeftOpen,
  PanelLeftClose,
  Sun,
  Moon,
  LayoutDashboard,
  Receipt,
  Target,
  BarChart3,
  Home,
  Wallet,
  PiggyBank,
  LogOut,
  Settings,
  X,
} from 'lucide-react';
import SettingsDialog from './SettingsDialog';
import type { User } from '@supabase/supabase-js';

const navigation = [
  { name: 'Dashboard', href: '/dashboard', icon: LayoutDashboard },
  { name: 'Expenses', href: '/expenses', icon: Receipt },
  { name: 'Budgets', href: '/budgets', icon: PiggyBank },
  { name: 'Goals', href: '/goals', icon: Target },
  { name: 'Analytics', href: '/analytics', icon: BarChart3 },
  { name: 'Households', href: '/households', icon: Home },
  { name: 'Settlements', href: '/settlements', icon: Wallet },
];

interface SidebarProps {
  onOpenRef?: (openFn: () => void) => void;
  onMobileStateChange?: (isOpen: boolean) => void;
}

export default function Sidebar({ onOpenRef, onMobileStateChange }: SidebarProps = {}) {
  const pathname = usePathname();
  const router = useRouter();
  const { theme, toggleTheme } = useTheme();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [isMobile, setIsMobile] = useState(false);
  const [isMobileOpen, setIsMobileOpen] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  // Expose open method via callback ref
  useEffect(() => {
    if (onOpenRef) {
      onOpenRef(() => setIsMobileOpen(true));
    }
  }, [onOpenRef]);

  // Notify parent of mobile sidebar state changes
  useEffect(() => {
    if (onMobileStateChange && isMobile) {
      onMobileStateChange(isMobileOpen);
    }
  }, [isMobileOpen, isMobile, onMobileStateChange]);

  useEffect(() => {
    const supabase = createClient();
    supabase.auth.getUser().then(({ data: { user } }) => {
      setUser(user);
      setLoading(false);
    });

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setUser(session?.user ?? null);
    });

    // Check localStorage for sidebar state
    const savedState = localStorage.getItem('sidebar-collapsed');
    if (savedState !== null) {
      setIsCollapsed(savedState === 'true');
    }

    // Check if mobile
    const checkMobile = () => {
      setIsMobile(window.innerWidth < 768);
      if (window.innerWidth >= 768) {
        setIsMobileOpen(false);
      }
    };
    checkMobile();
    window.addEventListener('resize', checkMobile);

    return () => {
      subscription.unsubscribe();
      window.removeEventListener('resize', checkMobile);
    };
  }, []);

  useEffect(() => {
    // Close mobile sidebar on route change
    if (isMobile) {
      setIsMobileOpen(false);
    }
  }, [pathname, isMobile]);

  const toggleSidebar = () => {
    const newState = !isCollapsed;
    setIsCollapsed(newState);
    localStorage.setItem('sidebar-collapsed', String(newState));
    // Dispatch custom event to notify MainLayout
    window.dispatchEvent(new Event('sidebar-state-changed'));
  };

  const handleLogout = async () => {
    if (isStandaloneApp()) return;
    const supabase = createClient();
    await supabase.auth.signOut();
    router.push('/auth/login');
    router.refresh();
  };

  const isAuthPage = pathname?.startsWith('/auth');
  const standalone = isStandaloneApp();

  if (isAuthPage || loading) {
    return null;
  }

  if (!user) {
    return null;
  }

  const sidebarContent = (
    <div
      className={`flex h-full flex-col bg-sidebar-bg text-sidebar-foreground transition-all duration-300 ${
        isCollapsed && !isMobile ? 'w-sidebar-collapsed' : 'w-sidebar'
      } ${isMobile ? 'fixed left-0 top-0 z-50' : 'fixed left-0 top-0 z-50'} ${isMobile && !isMobileOpen ? '-translate-x-full' : ''}`}
    >
      {/* Header */}
      <div className={`flex h-16 items-center justify-between border-b border-border ${isCollapsed && !isMobile ? 'px-2' : 'px-4'}`}>
        {!isCollapsed || isMobile ? (
          <Link href="/dashboard" className="flex items-center gap-2 flex-1">
            <Logo className="h-10 w-10" />
            <span className="text-lg font-semibold">TwoCents</span>
          </Link>
        ) : (
          <div className="flex flex-1 justify-center">
            <Logo className="h-10 w-10" />
          </div>
        )}
        <div className="flex items-center gap-2">
          {isMobile && (
            <button
              onClick={() => setIsMobileOpen(false)}
              className="inline-flex h-9 w-9 items-center justify-center rounded-notion border border-border/60 bg-card/50 backdrop-blur hover:bg-hover transition-colors"
              aria-label="Close sidebar"
            >
              <X className="h-5 w-5" />
            </button>
          )}
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 overflow-y-auto p-2">
        <ul className="space-y-1">
          {navigation.map((item) => {
            const isActive = pathname === item.href || pathname?.startsWith(item.href + '/');
            const Icon = item.icon;
            return (
              <li key={item.name}>
                <Link
                  href={item.href}
                  className={`flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                    isActive
                      ? 'bg-sidebar-active text-sidebar-foreground'
                      : 'text-muted-foreground hover:bg-sidebar-hover hover:text-sidebar-foreground'
                  } ${isCollapsed && !isMobile ? 'justify-center' : ''}`}
                  title={isCollapsed && !isMobile ? item.name : undefined}
                >
                  <Icon className="h-5 w-5 flex-shrink-0" />
                  {(!isCollapsed || isMobile) && <span>{item.name}</span>}
                </Link>
              </li>
            );
          })}
        </ul>
      </nav>

      {/* Footer */}
      <div className="border-t border-border p-2">
        {/* Collapse / expand (desktop) */}
        {!isMobile && (
          <button
            onClick={toggleSidebar}
            className={`mb-2 flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-sidebar-hover hover:text-sidebar-foreground ${
              isCollapsed ? 'justify-center' : ''
            }`}
            aria-label={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            title={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {isCollapsed ? (
              <PanelLeftOpen className="h-5 w-5 flex-shrink-0" />
            ) : (
              <PanelLeftClose className="h-5 w-5 flex-shrink-0" />
            )}
            {(!isCollapsed || isMobile) && <span>{isCollapsed ? 'Expand' : 'Collapse'}</span>}
          </button>
        )}

        {/* Settings Button */}
        <button
          onClick={() => setIsSettingsOpen(true)}
          className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-sidebar-hover hover:text-sidebar-foreground ${
            isCollapsed && !isMobile ? 'justify-center' : ''
          }`}
          title={isCollapsed && !isMobile ? 'Settings' : undefined}
        >
          <Settings className="h-5 w-5 flex-shrink-0" />
          {(!isCollapsed || isMobile) && <span>Settings</span>}
        </button>

        {!standalone && (
          <button
            type="button"
            onClick={handleLogout}
            className={`mt-2 flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-sidebar-hover hover:text-sidebar-foreground ${
              isCollapsed && !isMobile ? 'justify-center' : ''
            }`}
            title={isCollapsed && !isMobile ? 'Sign Out' : undefined}
          >
            <LogOut className="h-5 w-5 flex-shrink-0" />
            {(!isCollapsed || isMobile) && <span>Sign Out</span>}
          </button>
        )}
      </div>
    </div>
  );

  return (
    <>
      {/* Mobile overlay */}
      {isMobile && isMobileOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/50"
          onClick={() => setIsMobileOpen(false)}
        />
      )}

      {/* Sidebar */}
      {sidebarContent}
      
      <SettingsDialog 
        isOpen={isSettingsOpen} 
        onClose={() => setIsSettingsOpen(false)} 
      />
    </>
  );
}

