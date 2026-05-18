'use client';

import { useRouter } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import { isStandaloneApp } from '@/lib/standalone';
import { useEffect, useState } from 'react';
import type { User } from '@supabase/supabase-js';

export default function AuthButton() {
  const router = useRouter();
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

  if (loading) {
    return <div className="text-gray-600">Loading...</div>;
  }

  if (user) {
    return (
      <div className="flex items-center gap-4">
        {!isStandaloneApp() && (
          <>
            <span className="text-sm text-gray-700">{user.email}</span>
            <button
              type="button"
              onClick={handleLogout}
              className="rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700"
            >
              Sign out
            </button>
          </>
        )}
        {isStandaloneApp() && <span className="text-sm text-gray-700">TwoCents</span>}
      </div>
    );
  }

  if (isStandaloneApp()) {
    return null;
  }

  return (
    <button
      type="button"
      onClick={() => router.push('/auth/login')}
      className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700"
    >
      Sign in
    </button>
  );
}

