'use client';

import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
import type { ExpenseFlag } from '@twocents/shared';
import { formatDate } from '@twocents/shared';
import Button from '../components/ui/Button';
import { Flag, X } from 'lucide-react';

interface ExpenseFlagsProps {
  expenseId: string;
  onFlagChange?: () => void;
}

export default function ExpenseFlags({ expenseId, onFlagChange }: ExpenseFlagsProps) {
  const supabase = createClient();
  const [flags, setFlags] = useState<ExpenseFlag[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchFlags();
  }, [expenseId]);

  const fetchFlags = async () => {
    try {
      const { data, error } = await supabase
        .from('expense_flags')
        .select('*')
        .eq('expense_id', expenseId)
        .is('resolved_at', null)
        .order('created_at', { ascending: false });

      if (error) throw error;
      setFlags(data || []);
    } catch (error) {
      console.error('Error fetching flags:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleRemoveFlag = async (flagId: string) => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { error } = await supabase
        .from('expense_flags')
        .update({ resolved_at: new Date().toISOString() })
        .eq('id', flagId);

      if (error) throw error;
      await fetchFlags();
      onFlagChange?.();
    } catch (error) {
      console.error('Error removing flag:', error);
    }
  };

  if (loading || flags.length === 0) {
    return null;
  }

  return (
    <div className="flex flex-wrap gap-2">
      {flags.map((flag) => (
        <div
          key={flag.id}
          className="flex items-center gap-1 px-2 py-1 rounded text-xs bg-accent/10 text-accent"
        >
          <Flag className="h-3 w-3" />
          <span>{flag.flag_type}</span>
          <button
            onClick={() => handleRemoveFlag(flag.id)}
            className="ml-1 hover:bg-accent/20 rounded p-0.5"
          >
            <X className="h-3 w-3" />
          </button>
        </div>
      ))}
    </div>
  );
}

