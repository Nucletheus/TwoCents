'use client';

import { useState, useEffect, Suspense } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import Link from 'next/link';

function SignUpForm() {
  const signupsEnabled = process.env.NEXT_PUBLIC_ENABLE_SIGNUP === 'true';
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirect = searchParams.get('redirect');
  const prefillEmail = searchParams.get('email');

  const [email, setEmail] = useState(prefillEmail || '');
  const [password, setPassword] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    if (prefillEmail) {
      setEmail(prefillEmail);
    }
  }, [prefillEmail]);

  const handleSignUp = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSuccess(null);

    if (!signupsEnabled) {
      setError('Signups are currently disabled for private use. Ask the app owner to create your account.');
      return;
    }

    setLoading(true);

    const supabase = createClient();
    try {
      const { data, error } = await supabase.auth.signUp({
        email,
        password,
        options: {
          data: {
            name,
          },
        },
      });

      if (error) {
        setError(error.message);
        setLoading(false);
      } else {
        const { data: sessionData } = await supabase.auth.getSession();

        if (!sessionData?.session) {
          setError('Please check your email to confirm your account before signing in.');
          setLoading(false);
          return;
        }

        try {
          const { data: inviteResult } = await supabase.rpc('check_pending_invitations');
          if (inviteResult?.accepted_count > 0) {
            setSuccess(`Account created! You've been added to ${inviteResult.accepted_count} household(s).`);
          }
        } catch (err) {
          console.error('Failed to check invitations:', err);
        }

        if (redirect) {
          router.push(redirect);
        } else {
          router.push('/dashboard');
        }
        router.refresh();
      }
    } catch (networkError: any) {
      setError(networkError?.message || 'Failed to connect to server');
      setLoading(false);
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-gray-50">
      <div className="w-full max-w-md space-y-8 rounded-lg bg-white p-8 shadow-md">
        <div>
          <h2 className="mt-6 text-center text-3xl font-bold text-gray-900">Create your account</h2>
          {redirect?.includes('/invite/') && (
            <p className="mt-2 text-center text-sm text-gray-600">
              Sign up to accept your household invitation
            </p>
          )}
        </div>
        <form className="mt-8 space-y-6" onSubmit={handleSignUp}>
          {!signupsEnabled && (
            <div className="rounded-md bg-amber-50 p-4">
              <p className="text-sm text-amber-800">
                Account creation is disabled in private mode. Please sign in with an existing account.
              </p>
            </div>
          )}
          {error && (
            <div className="rounded-md bg-red-50 p-4">
              <p className="text-sm text-red-800">{error}</p>
            </div>
          )}
          {success && (
            <div className="rounded-md bg-green-50 p-4">
              <p className="text-sm text-green-800">{success}</p>
            </div>
          )}
          <div className="space-y-4">
            <div>
              <label htmlFor="name" className="block text-sm font-medium text-gray-700">
                Name
              </label>
              <input
                id="name"
                name="name"
                type="text"
                required
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="mt-1 block w-full rounded-md border border-gray-300 bg-white text-gray-900 px-3 py-2 shadow-sm focus:border-indigo-500 focus:outline-none focus:ring-indigo-500"
              />
            </div>
            <div>
              <label htmlFor="email" className="block text-sm font-medium text-gray-700">
                Email address
              </label>
              <input
                id="email"
                name="email"
                type="email"
                autoComplete="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="mt-1 block w-full rounded-md border border-gray-300 bg-white text-gray-900 px-3 py-2 shadow-sm focus:border-indigo-500 focus:outline-none focus:ring-indigo-500"
              />
              {prefillEmail && (
                <p className="mt-1 text-xs text-gray-500">This email was pre-filled from your invitation</p>
              )}
            </div>
            <div>
              <label htmlFor="password" className="block text-sm font-medium text-gray-700">
                Password
              </label>
              <input
                id="password"
                name="password"
                type="password"
                autoComplete="new-password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="mt-1 block w-full rounded-md border border-gray-300 bg-white text-gray-900 px-3 py-2 shadow-sm focus:border-indigo-500 focus:outline-none focus:ring-indigo-500"
              />
            </div>
          </div>

          <div>
            <button
              type="submit"
              disabled={loading || !signupsEnabled}
              className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50"
            >
              {loading ? 'Creating account...' : signupsEnabled ? 'Sign up' : 'Signups disabled'}
            </button>
          </div>

          <div className="text-center">
            <Link
              href={redirect ? `/auth/login?redirect=${encodeURIComponent(redirect)}` : '/auth/login'}
              className="text-sm text-indigo-600 hover:text-indigo-500"
            >
              Already have an account? Sign in
            </Link>
          </div>
        </form>
      </div>
    </div>
  );
}

export default function SignUpFormClient() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-screen items-center justify-center bg-gray-50">
          <div className="text-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 mx-auto" />
            <p className="mt-4 text-gray-600">Loading...</p>
          </div>
        </div>
      }
    >
      <SignUpForm />
    </Suspense>
  );
}
