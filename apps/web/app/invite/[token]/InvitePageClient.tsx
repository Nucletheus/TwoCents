'use client';

import { useState, useEffect, use } from 'react';
import { useRouter } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import Link from 'next/link';

interface InvitePageClientProps {
  params: Promise<{ token: string }>;
}

export default function InvitePageClient({ params }: InvitePageClientProps) {
  const { token } = use(params);
  const router = useRouter();
  const [loading, setLoading] = useState(true);
  const [accepting, setAccepting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [invitation, setInvitation] = useState<{
    email: string;
    household_name: string;
    inviter_name: string;
    role: string;
    expires_at: string;
  } | null>(null);
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [userEmail, setUserEmail] = useState<string | null>(null);

  useEffect(() => {
    checkInvitation();
  }, [token]);

  const checkInvitation = async () => {
    setLoading(true);
    const supabase = createClient();

    try {
      const { data: { user } } = await supabase.auth.getUser();
      setIsLoggedIn(!!user);
      setUserEmail(user?.email || null);

      const { data, error: rpcError } = await supabase.rpc('get_invitation_details', {
        invitation_token: token,
      });

      if (rpcError) {
        setError('This invitation is invalid or has expired.');
        setLoading(false);
        return;
      }

      if (data) {
        setInvitation(data);
      } else {
        setError('This invitation is invalid or has expired.');
      }
    } catch (err) {
      console.error('Error checking invitation:', err);
      setError('Unable to verify invitation.');
    } finally {
      setLoading(false);
    }
  };

  const handleAcceptInvitation = async () => {
    setAccepting(true);
    setError(null);
    const supabase = createClient();

    try {
      const { data, error: acceptError } = await supabase.rpc('accept_household_invitation', {
        invitation_token: token,
      });

      if (acceptError) {
        setError(acceptError.message);
        return;
      }

      const result = data as { success: boolean; message?: string; error?: string; household_id?: string };

      if (result.success) {
        setSuccess(result.message || 'Successfully joined household!');
        setTimeout(() => {
          router.push(`/households/${result.household_id}`);
        }, 2000);
      } else {
        setError(result.error || 'Failed to accept invitation');
      }
    } catch (err) {
      console.error('Error accepting invitation:', err);
      setError('Failed to accept invitation. Please try again.');
    } finally {
      setAccepting(false);
    }
  };

  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 mx-auto" />
          <p className="mt-4 text-gray-600">Verifying invitation...</p>
        </div>
      </div>
    );
  }

  if (error && !invitation) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-50">
        <div className="w-full max-w-md space-y-6 rounded-lg bg-white p-8 shadow-md text-center">
          <div className="mx-auto h-16 w-16 rounded-full bg-red-100 flex items-center justify-center">
            <svg className="h-8 w-8 text-red-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </div>
          <h2 className="text-2xl font-bold text-gray-900">Invalid Invitation</h2>
          <p className="text-gray-600">{error}</p>
          <Link
            href="/dashboard"
            className="inline-block mt-4 px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700"
          >
            Go to Dashboard
          </Link>
        </div>
      </div>
    );
  }

  if (success) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-50">
        <div className="w-full max-w-md space-y-6 rounded-lg bg-white p-8 shadow-md text-center">
          <div className="mx-auto h-16 w-16 rounded-full bg-green-100 flex items-center justify-center">
            <svg className="h-8 w-8 text-green-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
          </div>
          <h2 className="text-2xl font-bold text-gray-900">Welcome to the Household!</h2>
          <p className="text-gray-600">{success}</p>
          <p className="text-sm text-gray-500">Redirecting you now...</p>
        </div>
      </div>
    );
  }

  if (!isLoggedIn) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-50">
        <div className="w-full max-w-md space-y-6 rounded-lg bg-white p-8 shadow-md">
          <div className="text-center">
            <div className="mx-auto h-16 w-16 rounded-full bg-indigo-100 flex items-center justify-center">
              <svg className="h-8 w-8 text-indigo-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z"
                />
              </svg>
            </div>
            <h2 className="mt-4 text-2xl font-bold text-gray-900">You&apos;re Invited!</h2>
            {invitation && (
              <p className="mt-2 text-gray-600">
                <strong>{invitation.inviter_name}</strong> invited you to join <strong>{invitation.household_name}</strong>
              </p>
            )}
          </div>

          <div className="space-y-4">
            <p className="text-center text-sm text-gray-600">Create an account or sign in to accept this invitation.</p>

            <Link
              href={`/auth/signup?redirect=/invite/${token}&email=${encodeURIComponent(invitation?.email || '')}`}
              className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700"
            >
              Create Account
            </Link>

            <Link
              href={`/auth/login?redirect=/invite/${token}`}
              className="w-full flex justify-center py-2 px-4 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
            >
              Sign In
            </Link>
          </div>

          {invitation && (
            <div className="mt-6 rounded-lg bg-gray-50 p-4">
              <p className="text-xs text-gray-500 text-center">
                Invitation for <strong>{invitation.email}</strong>
                <br />
                Role: <span className="capitalize">{invitation.role}</span>
                <br />
                Expires: {new Date(invitation.expires_at).toLocaleDateString()}
              </p>
            </div>
          )}
        </div>
      </div>
    );
  }

  const emailMatches = invitation?.email.toLowerCase() === userEmail?.toLowerCase();

  return (
    <div className="flex min-h-screen items-center justify-center bg-gray-50">
      <div className="w-full max-w-md space-y-6 rounded-lg bg-white p-8 shadow-md">
        <div className="text-center">
          <div className="mx-auto h-16 w-16 rounded-full bg-indigo-100 flex items-center justify-center">
            <svg className="h-8 w-8 text-indigo-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z"
              />
            </svg>
          </div>
          <h2 className="mt-4 text-2xl font-bold text-gray-900">Accept Invitation</h2>
          {invitation && (
            <p className="mt-2 text-gray-600">
              <strong>{invitation.inviter_name}</strong> invited you to join <strong>{invitation.household_name}</strong>
            </p>
          )}
        </div>

        {!emailMatches && (
          <div className="rounded-md bg-yellow-50 p-4">
            <p className="text-sm text-yellow-800">
              This invitation was sent to <strong>{invitation?.email}</strong>, but you&apos;re signed in as{' '}
              <strong>{userEmail}</strong>.
            </p>
          </div>
        )}

        {error && (
          <div className="rounded-md bg-red-50 p-4">
            <p className="text-sm text-red-800">{error}</p>
          </div>
        )}

        <div className="space-y-4">
          <button
            type="button"
            onClick={handleAcceptInvitation}
            disabled={accepting}
            className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50"
          >
            {accepting ? 'Accepting...' : 'Accept Invitation'}
          </button>

          <Link
            href="/dashboard"
            className="w-full flex justify-center py-2 px-4 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
          >
            Decline & Go to Dashboard
          </Link>
        </div>

        {invitation && (
          <div className="mt-6 rounded-lg bg-gray-50 p-4">
            <p className="text-xs text-gray-500 text-center">
              You&apos;ll join as: <span className="capitalize font-medium">{invitation.role}</span>
              <br />
              Expires: {new Date(invitation.expires_at).toLocaleDateString()}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
