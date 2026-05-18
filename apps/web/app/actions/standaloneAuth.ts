'use server';

import { createServerClient } from '@supabase/ssr';
import { cookies } from 'next/headers';
import { getSupabaseEnv } from '@/lib/env';

export type StandaloneSignInResult = { ok: true } | { ok: false; error: string };

/**
 * Signs in using server-only env credentials. For local/standalone; never set creds on a public host.
 */
export async function signInStandaloneSession(): Promise<StandaloneSignInResult> {
  const email = process.env.STANDALONE_AUTH_EMAIL;
  const password = process.env.STANDALONE_AUTH_PASSWORD;
  if (!email || !password) {
    return {
      ok: false,
      error:
        'Set STANDALONE_AUTH_EMAIL and STANDALONE_AUTH_PASSWORD in apps/web/.env.local (and create that user in local Supabase).',
    };
  }

  const cookieStore = await cookies();
  const { url, anonKey } = getSupabaseEnv();

  const supabase = createServerClient(url, anonKey, {
    cookies: {
      get(name: string) {
        return cookieStore.get(name)?.value;
      },
      set(
        name: string,
        value: string,
        options: { path?: string; maxAge?: number; domain?: string; sameSite?: 'lax' | 'strict' | 'none' }
      ) {
        try {
          cookieStore.set({ name, value, ...options });
        } catch {
          // can fail in some server contexts; middleware will refresh
        }
      },
      remove(name: string, options: { path?: string; maxAge?: number; domain?: string; sameSite?: 'lax' | 'strict' | 'none' }) {
        try {
          cookieStore.set({ name, value: '', ...options, maxAge: 0 });
        } catch {
          // ignore
        }
      },
    },
  });

  const { error } = await supabase.auth.signInWithPassword({ email, password });
  if (error) {
    return { ok: false, error: error.message };
  }
  return { ok: true };
}
