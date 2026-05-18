'use client';

import { useEffect, useState } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import { isStandaloneApp } from '@/lib/standalone';
import Link from 'next/link';
import type { User } from '@supabase/supabase-js';

export default function Navigation() {
  const router = useRouter();
  const pathname = usePathname();
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

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

  const handleLogout = async () => {
    if (isStandaloneApp()) return;
    const supabase = createClient();
    await supabase.auth.signOut();
    router.push('/auth/login');
    router.refresh();
  };

  const isAuthPage = pathname?.startsWith('/auth');

  if (isAuthPage || loading) {
    return null;
  }

  if (!user) {
    if (isStandaloneApp()) {
      return null;
    }
    return (
      <nav className="bg-white shadow">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="flex h-16 items-center justify-between">
            <Link href="/" className="text-xl font-bold text-indigo-600">
              TwoCents
            </Link>
            <Link
              href="/auth/login"
              className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700"
            >
              Sign In
            </Link>
          </div>
        </div>
      </nav>
    );
  }

  return (
    <nav className="bg-white shadow">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="flex h-16 items-center justify-between">
          <div className="flex items-center gap-8">
            <Link href="/dashboard" className="text-xl font-bold text-indigo-600">
              TwoCents
            </Link>
            <div className="flex gap-4">
              <Link
                href="/dashboard"
                className={`px-3 py-2 text-sm font-medium ${
                  pathname === '/dashboard'
                    ? 'text-indigo-600 border-b-2 border-indigo-600'
                    : 'text-gray-700 hover:text-indigo-600'
                }`}
              >
                Dashboard
              </Link>
              <Link
                href="/expenses"
                className={`px-3 py-2 text-sm font-medium ${
                  pathname?.startsWith('/expenses')
                    ? 'text-indigo-600 border-b-2 border-indigo-600'
                    : 'text-gray-700 hover:text-indigo-600'
                }`}
              >
                Expenses
              </Link>
              <Link
                href="/goals"
                className={`px-3 py-2 text-sm font-medium ${
                  pathname?.startsWith('/goals')
                    ? 'text-indigo-600 border-b-2 border-indigo-600'
                    : 'text-gray-700 hover:text-indigo-600'
                }`}
              >
                Goals
              </Link>
              <Link
                href="/analytics"
                className={`px-3 py-2 text-sm font-medium ${
                  pathname?.startsWith('/analytics')
                    ? 'text-indigo-600 border-b-2 border-indigo-600'
                    : 'text-gray-700 hover:text-indigo-600'
                }`}
              >
                Analytics
              </Link>
              <Link
                href="/households"
                className={`px-3 py-2 text-sm font-medium ${
                  pathname?.startsWith('/households')
                    ? 'text-indigo-600 border-b-2 border-indigo-600'
                    : 'text-gray-700 hover:text-indigo-600'
                }`}
              >
                Households
              </Link>
              <Link
                href="/settlements"
                className={`px-3 py-2 text-sm font-medium ${
                  pathname?.startsWith('/settlements')
                    ? 'text-indigo-600 border-b-2 border-indigo-600'
                    : 'text-gray-700 hover:text-indigo-600'
                }`}
              >
                Settlements
              </Link>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <span className="text-sm text-gray-700">{user.email}</span>
            {!isStandaloneApp() && (
              <button
                type="button"
                onClick={handleLogout}
                className="rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700"
              >
                Sign Out
              </button>
            )}
          </div>
        </div>
      </div>
    </nav>
  );
}

