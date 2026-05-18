'use client';

import { useState, useEffect } from 'react';
import { usePathname } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import { Plus } from 'lucide-react';
import Button from './ui/Button';
import QuickActionDialog from './QuickActionDialog';
import type { User } from '@supabase/supabase-js';
import { useHousehold } from './HouseholdProvider';

export default function QuickActionBar() {
  const pathname = usePathname();
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const { selectedHouseholdId } = useHousehold();

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

    return () => subscription.unsubscribe();
  }, []);

  const isAuthPage = pathname?.startsWith('/auth');
  const isLandingPage = pathname === '/';

  if (isAuthPage || isLandingPage || loading || !user) {
    return null;
  }

  const handleOpenDialog = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDialogOpen(true);
  };

  const handleCloseDialog = () => {
    setIsDialogOpen(false);
  };

  return (
    <>
      <footer className="border-t border-border/60 backdrop-blur-md bg-card/50 shadow-notion-sm">
        <div className="mx-auto max-w-7xl px-4">
          <div className="flex h-16 items-center justify-center">
            <Button
              onClick={handleOpenDialog}
              size="lg"
              className="rounded-full h-12 w-12 p-0 shadow-notion-sm hover:shadow-notion"
              aria-label="Quick Actions"
              type="button"
            >
              <Plus className="h-6 w-6" />
            </Button>
          </div>
        </div>
      </footer>

      {/* Quick Action Dialog */}
      <QuickActionDialog
        isOpen={isDialogOpen}
        onClose={handleCloseDialog}
        selectedHouseholdId={selectedHouseholdId}
      />
    </>
  );
}

