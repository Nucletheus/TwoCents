'use client';

import { useCallback, useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { signInStandaloneSession } from '@/app/actions/standaloneAuth';

/**
 * Renders while standalone mode establishes the hidden Supabase session (RLS + anon API still use JWT).
 */
export function ConnectLocalSession() {
  const router = useRouter();
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async () => {
    setError(null);
    const res = await signInStandaloneSession();
    if (res.ok) {
      router.refresh();
      return;
    }
    setError(res.error);
  }, [router]);

  useEffect(() => {
    void run();
  }, [run]);

  if (error) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background p-6">
        <div className="max-w-md rounded-lg border border-destructive/30 bg-destructive/5 p-6 text-sm text-foreground">
          <p className="font-semibold text-destructive">Could not start the app</p>
          <p className="mt-2 text-muted-foreground">{error}</p>
          <p className="mt-4 text-xs text-muted-foreground">
            Create a user in local Supabase (Authentication → Add user) with the same email/password as
            STANDALONE_AUTH_* in apps/web/.env.local, or follow README setup, then refresh.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-background gap-3">
      <div className="h-8 w-8 animate-spin rounded-full border-2 border-indigo-600 border-t-transparent" />
      <p className="text-sm text-muted-foreground">Starting TwoCents…</p>
    </div>
  );
}
