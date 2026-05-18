'use client';

import { useEffect, useState } from 'react';
import { createClient } from '@/lib/supabase/client';
import type { TransactionRule, Expense } from '@twocents/shared';
import { applyTransactionRules } from '@twocents/shared';

interface TransactionRuleMatcherProps {
  householdId: string | null;
  expense: Partial<Expense>;
  onCategoryMatched: (categoryId: string) => void;
}

export function useTransactionRules(householdId: string | null) {
  const supabase = createClient();
  const [rules, setRules] = useState<TransactionRule[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (householdId) {
      fetchRules();
    } else {
      setRules([]);
      setLoading(false);
    }
  }, [householdId]);

  const fetchRules = async () => {
    if (!householdId) return;

    try {
      const { data, error } = await supabase
        .from('transaction_rules')
        .select('*')
        .eq('household_id', householdId)
        .eq('is_active', true)
        .order('priority', { ascending: false });

      if (error) throw error;
      setRules(data || []);
    } catch (error) {
      console.error('Error fetching transaction rules:', error);
      setRules([]);
    } finally {
      setLoading(false);
    }
  };

  return { rules, loading, refetch: fetchRules };
}

export function matchExpenseToRule(expense: Partial<Expense>, rules: TransactionRule[]): string | null {
  if (!expense.date || expense.amount === undefined) {
    return null;
  }

  const fullExpense: Expense = {
    id: expense.id || '',
    household_id: expense.household_id || '',
    payer_id: expense.payer_id || '',
    amount: expense.amount,
    category_id: expense.category_id || '',
    description: expense.description || null,
    vendor: expense.vendor || null,
    status: expense.status || 'approved',
    reviewed_by: expense.reviewed_by || null,
    reviewed_at: expense.reviewed_at || null,
    date: expense.date,
    receipt_url: expense.receipt_url || null,
    created_at: expense.created_at || new Date().toISOString(),
  };

  return applyTransactionRules(fullExpense, rules);
}

