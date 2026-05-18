'use client';

import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { createClient } from '@/lib/supabase/client';
import { useHouseholds, type Household } from '@twocents/shared';

type HouseholdContextValue = {
  households: Household[];
  selectedHouseholdId: string | null;
  selectedHousehold: Household | null;
  setSelectedHouseholdId: (householdId: string | null) => void;
  loading: boolean;
  error: Error | null;
  refetch: () => void;
};

const HouseholdContext = createContext<HouseholdContextValue | undefined>(undefined);

const STORAGE_KEY = 'twocents:selectedHouseholdId';

export function HouseholdProvider({ children }: { children: React.ReactNode }) {
  const supabase = useMemo(() => createClient(), []);
  const { households, loading, error, refetch } = useHouseholds(supabase);

  // Initialize with null to avoid hydration mismatch, then sync from localStorage
  const [selectedHouseholdId, setSelectedHouseholdIdState] = useState<string | null>(null);
  const [isHydrated, setIsHydrated] = useState(false);

  // Hydrate from localStorage after mount
  useEffect(() => {
    if (typeof window === 'undefined') return;
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        setSelectedHouseholdIdState(stored);
      }
    } catch {
      // ignore storage errors
    } finally {
      setIsHydrated(true);
    }
  }, []);

  const setSelectedHouseholdId = useCallback((householdId: string | null) => {
    setSelectedHouseholdIdState(householdId);
  }, []);

  // Persist selection
  useEffect(() => {
    if (!isHydrated || typeof window === 'undefined') return;
    try {
      if (selectedHouseholdId) {
        localStorage.setItem(STORAGE_KEY, selectedHouseholdId);
      } else {
        localStorage.removeItem(STORAGE_KEY);
      }
    } catch {
      // ignore storage errors (private mode, disabled storage, etc.)
    }
  }, [selectedHouseholdId, isHydrated]);

  // Ensure selection is valid when household list changes
  useEffect(() => {
    if (loading || !isHydrated) return;

    if (households.length === 0) {
      if (selectedHouseholdId !== null) {
        setSelectedHouseholdIdState(null);
      }
      return;
    }

    const isValid =
      selectedHouseholdId !== null && households.some((h) => h.id === selectedHouseholdId);

    if (!isValid) {
      // Only update if we don't have a valid selection
      const firstHouseholdId = households[0]?.id;
      if (firstHouseholdId && selectedHouseholdId !== firstHouseholdId) {
        setSelectedHouseholdIdState(firstHouseholdId);
      }
    }
  }, [households, loading, selectedHouseholdId, isHydrated]);

  // Stabilize selectedHousehold by using householdsKey instead of households array
  const householdsKey = households.map(h => h.id).join(',');
  const selectedHousehold = useMemo(
    () => households.find((h) => h.id === selectedHouseholdId) ?? null,
    [householdsKey, selectedHouseholdId, households] // Include householdsKey for stability, but still need households for the find
  );

  // Stabilize refetch function reference using useRef to prevent unnecessary context updates
  const refetchRef = useRef(refetch);
  useEffect(() => {
    refetchRef.current = refetch;
  }, [refetch]);
  
  const stableRefetch = useCallback(() => {
    refetchRef.current();
  }, []);

  // Use a ref to store the context value - this ensures the reference never changes
  // We'll update the object properties in place, which is safe for context values
  const valueRef = useRef<HouseholdContextValue>({
    households: [],
    selectedHouseholdId: null,
    selectedHousehold: null,
    setSelectedHouseholdId: () => {},
    loading: true,
    error: null,
    refetch: () => {},
  });
  
  // Track what we last set to detect changes
  const lastSetRef = useRef({
    householdsKey: '',
    selectedHouseholdId: null as string | null,
    selectedHouseholdIdValue: 'null' as string,
    loading: true,
    errorMessage: 'null',
  });
  
  // Update the ref's properties synchronously during render
  // This is safe because we're not changing the object reference
  const currentKey = `${householdsKey}|${selectedHouseholdId}|${selectedHousehold?.id ?? 'null'}|${loading}|${error?.message ?? 'null'}`;
  const lastKey = `${lastSetRef.current.householdsKey}|${lastSetRef.current.selectedHouseholdId}|${lastSetRef.current.selectedHouseholdIdValue}|${lastSetRef.current.loading}|${lastSetRef.current.errorMessage}`;
  
  if (currentKey !== lastKey) {
    valueRef.current.households = households;
    valueRef.current.selectedHouseholdId = selectedHouseholdId;
    valueRef.current.selectedHousehold = selectedHousehold;
    valueRef.current.setSelectedHouseholdId = setSelectedHouseholdId;
    valueRef.current.loading = loading;
    valueRef.current.error = error;
    valueRef.current.refetch = stableRefetch;
    lastSetRef.current = {
      householdsKey,
      selectedHouseholdId,
      selectedHouseholdIdValue: selectedHousehold?.id ?? 'null',
      loading,
      errorMessage: error?.message ?? 'null',
    };
  }
  
  const value = valueRef.current;

  return <HouseholdContext.Provider value={value}>{children}</HouseholdContext.Provider>;
}

export function useHousehold() {
  const ctx = useContext(HouseholdContext);
  if (!ctx) {
    throw new Error('useHousehold must be used within a HouseholdProvider');
  }
  return ctx;
}


