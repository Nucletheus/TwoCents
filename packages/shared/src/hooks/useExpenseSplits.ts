import { useState, useEffect } from 'react';
import type { ExpenseSplit } from '../types';
import { calculateSplitAmount } from '../utils';

export function useExpenseSplits(supabase: any, expenseId: string | null) {
  const [splits, setSplits] = useState<ExpenseSplit[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (expenseId) {
      fetchSplits();
    }
  }, [expenseId]);

  const fetchSplits = async () => {
    if (!expenseId) return;

    try {
      const { data, error } = await supabase
        .from('expense_splits')
        .select(
          `
          *,
          profiles:user_id (
            id,
            name,
            email
          )
        `
        )
        .eq('expense_id', expenseId);

      if (error) throw error;
      setSplits(data || []);
    } catch (err: any) {
      setError(err);
    } finally {
      setLoading(false);
    }
  };

  const createSplits = async (
    expenseId: string,
    splitType: 'equal' | 'percentage' | 'custom',
    userIds: string[],
    totalAmount: number,
    percentages?: number[],
    customAmounts?: number[]
  ) => {
    try {
      // Delete existing splits
      await supabase.from('expense_splits').delete().eq('expense_id', expenseId);

      const amounts = calculateSplitAmount(
        totalAmount,
        splitType,
        userIds.length,
        percentages,
        customAmounts
      );

      const splitsToInsert = userIds.map((userId, index) => ({
        expense_id: expenseId,
        user_id: userId,
        amount: amounts[index],
        percentage: splitType === 'percentage' ? percentages?.[index] || null : null,
      }));

      const { data, error } = await supabase
        .from('expense_splits')
        .insert(splitsToInsert)
        .select();

      if (error) throw error;
      await fetchSplits();
      return data;
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  const deleteSplits = async (expenseId: string) => {
    try {
      const { error } = await supabase
        .from('expense_splits')
        .delete()
        .eq('expense_id', expenseId);

      if (error) throw error;
      await fetchSplits();
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  return {
    splits,
    loading,
    error,
    createSplits,
    deleteSplits,
    refetch: fetchSplits,
  };
}

export async function calculateSettlements(supabase: any, householdId: string) {
  try {
    // Get all expenses and their splits for the household
    const { data: expenses, error: expensesError } = await supabase
      .from('expenses')
      .select(
        `
        id,
        amount,
        payer_id,
        expense_splits (
          user_id,
          amount
        )
      `
      )
      .eq('household_id', householdId);

    if (expensesError) throw expensesError;

    // Get all household members
    const { data: members, error: membersError } = await supabase
      .from('household_members')
      .select('user_id')
      .eq('household_id', householdId);

    if (membersError) throw membersError;

    const userIds = members?.map((m) => m.user_id) || [];

    // Calculate net balances
    const balances: Record<string, number> = {};
    userIds.forEach((userId) => {
      balances[userId] = 0;
    });

    expenses?.forEach((expense: any) => {
      // Payer paid the full amount
      balances[expense.payer_id] = (balances[expense.payer_id] || 0) + expense.amount;

      // Split amounts are what each person owes
      expense.expense_splits?.forEach((split: any) => {
        balances[split.user_id] = (balances[split.user_id] || 0) - split.amount;
      });
    });

    // Calculate who owes whom
    const settlements: Array<{
      from: string;
      to: string;
      amount: number;
    }> = [];

    const positiveBalances = Object.entries(balances)
      .filter(([_, balance]) => balance > 0.01)
      .sort(([_, a], [__, b]) => b - a);

    const negativeBalances = Object.entries(balances)
      .filter(([_, balance]) => balance < -0.01)
      .sort(([_, a], [__, b]) => a - b);

    let posIndex = 0;
    let negIndex = 0;

    while (posIndex < positiveBalances.length && negIndex < negativeBalances.length) {
      const [fromUserId, fromBalance] = negativeBalances[negIndex];
      const [toUserId, toBalance] = positiveBalances[posIndex];

      const amount = Math.min(Math.abs(fromBalance), toBalance);

      settlements.push({
        from: fromUserId,
        to: toUserId,
        amount: parseFloat(amount.toFixed(2)),
      });

      negativeBalances[negIndex][1] = fromBalance + amount;
      positiveBalances[posIndex][1] = toBalance - amount;

      if (Math.abs(negativeBalances[negIndex][1]) < 0.01) {
        negIndex++;
      }
      if (positiveBalances[posIndex][1] < 0.01) {
        posIndex++;
      }
    }

    // Get user names for settlements
    const { data: profiles } = await supabase
      .from('profiles')
      .select('id, name, email')
      .in('id', userIds);

    const profileMap = new Map(profiles?.map((p) => [p.id, p]) || []);

    return settlements.map((settlement) => ({
      ...settlement,
      fromName: profileMap.get(settlement.from)?.name || 'Unknown',
      toName: profileMap.get(settlement.to)?.name || 'Unknown',
    }));
  } catch (err: any) {
    throw err;
  }
}

