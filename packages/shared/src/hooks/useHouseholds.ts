import { useState, useEffect, useCallback } from 'react';
import type { Household, HouseholdMember } from '../types';

export function useHouseholds(supabase: any) {
  const [households, setHouseholds] = useState<Household[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchHouseholds = useCallback(async (userId?: string) => {
    setLoading(true);
    try {
      // If userId is provided (from auth state change), use it directly
      // Otherwise, try to get the user from the session
      let user;
      if (userId) {
        user = { id: userId };
      } else {
        // Try getSession first as it's more reliable right after sign-in
        const { data: { session } } = await supabase.auth.getSession();
        if (session?.user) {
          user = session.user;
        } else {
          // Fall back to getUser if getSession doesn't have a user
          const {
            data: { user: getUserResult },
          } = await supabase.auth.getUser();
          user = getUserResult;
        }
      }

      if (!user) {
        setHouseholds([]);
        setLoading(false);
        return;
      }

      const { data, error } = await supabase
        .from('household_members')
        .select(
          `
          household_id,
          households (
            id,
            name,
            created_at
          )
        `
        )
        .eq('user_id', user.id);

      if (error) throw error;

      const householdList =
        data?.map((member: any) => member.households).filter(Boolean) || [];
      
      // Only update if the household IDs actually changed to prevent unnecessary re-renders
      setHouseholds((prevHouseholds) => {
        const prevIds = prevHouseholds.map((h) => h.id).sort().join(',');
        const newIds = householdList.map((h: any) => h.id).sort().join(',');
        if (prevIds === newIds && prevHouseholds.length === householdList.length) {
          // Return previous array reference if IDs haven't changed
          return prevHouseholds;
        }
        return householdList;
      });
    } catch (err: any) {
      setError(err);
    } finally {
      setLoading(false);
    }
  }, [supabase]);

  useEffect(() => {
    // Keep household list in sync with auth state.
    // This prevents a race where getUser() briefly returns null right after sign-in,
    // which would otherwise leave households empty until a full reload.
    const { data } = supabase.auth.onAuthStateChange((event: string, session: any) => {
      if (event === 'INITIAL_SESSION' || event === 'SIGNED_IN' || event === 'TOKEN_REFRESHED' || event === 'USER_UPDATED') {
        // Use the session's user ID directly to avoid race conditions
        const userId = session?.user?.id;
        if (userId) {
          void fetchHouseholds(userId);
        } else {
          // Fallback to regular fetch if no session user
          void fetchHouseholds();
        }
      }
      if (event === 'SIGNED_OUT') {
        setHouseholds([]);
        setLoading(false);
      }
    });

    // Also fetch immediately on mount (covers cases where INITIAL_SESSION doesn't fire for some reason).
    void fetchHouseholds();

    return () => {
      data?.subscription?.unsubscribe?.();
    };
  }, [fetchHouseholds, supabase]);

  const createHousehold = useCallback(async (name: string) => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();

      if (!user) throw new Error('Not authenticated');

      const { data: sessionData, error: sessionError } = await supabase.auth.getSession();
      void sessionData;
      void sessionError;
      const { data: household, error: householdError } = await supabase.rpc('create_household', {
        p_name: name,
      });

      if (householdError) throw householdError;

      await fetchHouseholds();
      return household;
    } catch (err: any) {
      setError(err);
      throw err;
    }
  }, [supabase, fetchHouseholds]);

  const deleteHousehold = useCallback(async (householdId: string) => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();

      if (!user) throw new Error('Not authenticated');

      const { error } = await supabase.rpc('delete_household', {
        p_household_id: householdId,
      });

      if (error) throw error;

      await fetchHouseholds();
    } catch (err: any) {
      setError(err);
      throw err;
    }
  }, [supabase, fetchHouseholds]);

  const inviteMember = useCallback(async (householdId: string, email: string) => {
    try {
      // In a real app, you'd send an email invitation
      // For now, we'll just return a success message
      // This would typically involve:
      // 1. Looking up the user by email
      // 2. Adding them to household_members if they exist
      // 3. Sending an invitation email if they don't exist
      return { success: true, message: 'Invitation sent' };
    } catch (err: any) {
      setError(err);
      throw err;
    }
  }, []);

  return {
    households,
    loading,
    error,
    createHousehold,
    deleteHousehold,
    inviteMember,
    refetch: fetchHouseholds,
  };
}

