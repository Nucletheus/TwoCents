import { useState, useEffect, useCallback } from 'react';
import type { SavingsGoal, GoalContribution, UnallocatedSavings } from '../types';

export function useSavingsGoals(supabase: any, householdId: string | null) {
  const [goals, setGoals] = useState<SavingsGoal[]>([]);
  const [contributions, setContributions] = useState<Record<string, GoalContribution[]>>({});
  const [unallocatedSavings, setUnallocatedSavings] = useState<UnallocatedSavings[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (householdId) {
      fetchGoals();
      fetchUnallocatedSavings();
    } else {
      setGoals([]);
      setContributions({});
      setUnallocatedSavings([]);
      setLoading(false);
    }
  }, [householdId]);

  const fetchGoals = async () => {
    if (!householdId) return;

    try {
      setLoading(true);
      const { data, error } = await supabase
        .from('savings_goals')
        .select('*')
        .eq('household_id', householdId)
        .order('priority', { ascending: true });

      if (error) throw error;
      setGoals(data || []);

      // Fetch contributions for all goals
      if (data && data.length > 0) {
        const goalIds = data.map((g: SavingsGoal) => g.id);
        const { data: contribs, error: contribError } = await supabase
          .from('goal_contributions')
          .select('*')
          .in('goal_id', goalIds)
          .order('date', { ascending: false });

        if (!contribError && contribs) {
          const contribsByGoal: Record<string, GoalContribution[]> = {};
          contribs.forEach((contrib: GoalContribution) => {
            if (!contribsByGoal[contrib.goal_id]) {
              contribsByGoal[contrib.goal_id] = [];
            }
            contribsByGoal[contrib.goal_id].push(contrib);
          });
          setContributions(contribsByGoal);
        }
      } else {
        setContributions({});
      }
    } catch (err: any) {
      setError(err);
    } finally {
      setLoading(false);
    }
  };

  const fetchUnallocatedSavings = async () => {
    if (!householdId) return;

    try {
      const { data, error } = await supabase
        .from('unallocated_savings')
        .select('*')
        .eq('household_id', householdId)
        .is('allocated_at', null)
        .order('date', { ascending: false });

      if (error) throw error;
      setUnallocatedSavings(data || []);
    } catch (err: any) {
      console.error('Error fetching unallocated savings:', err);
    }
  };

  const createGoal = async (goalData: {
    household_id: string;
    name: string;
    target_amount: number;
    deadline?: string;
    account_id?: string | null;
  }) => {
    try {
      // Get the max priority to put new goal at end
      const maxPriority = goals.length > 0 
        ? Math.max(...goals.map(g => g.priority || 0)) + 1 
        : 0;

      const { data, error } = await supabase
        .from('savings_goals')
        .insert({
          ...goalData,
          priority: maxPriority,
        })
        .select()
        .single();

      if (error) throw error;
      
      // Optimistically add the new goal to the list
      if (data) {
        setGoals(prev => [...prev, data].sort((a, b) => (a.priority || 0) - (b.priority || 0)));
      }
      
      return data;
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  const updateGoal = async (id: string, updates: Partial<SavingsGoal>) => {
    // Store previous state for rollback
    const previousGoals = [...goals];
    
    try {
      // Optimistically update the local state
      setGoals(prev => prev.map(g => g.id === id ? { ...g, ...updates } : g));
      
      const { error } = await supabase
        .from('savings_goals')
        .update(updates)
        .eq('id', id);

      if (error) {
        // Rollback on error
        setGoals(previousGoals);
        throw error;
      }
    } catch (err: any) {
      setGoals(previousGoals);
      setError(err);
      throw err;
    }
  };

  const deleteGoal = async (id: string) => {
    // Store previous state for rollback
    const previousGoals = [...goals];
    const previousContributions = { ...contributions };
    
    try {
      // Optimistically remove the goal from the list
      setGoals(prev => prev.filter(g => g.id !== id));
      setContributions(prev => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      
      const { error } = await supabase.from('savings_goals').delete().eq('id', id);

      if (error) {
        // Rollback on error
        setGoals(previousGoals);
        setContributions(previousContributions);
        throw error;
      }
    } catch (err: any) {
      // Rollback on error
      setGoals(previousGoals);
      setContributions(previousContributions);
      setError(err);
      throw err;
    }
  };

  // Reorder goals by priority (for drag-and-drop)
  const reorderGoals = async (orderedGoalIds: string[]) => {
    try {
      // Optimistically update local state
      const reorderedGoals = orderedGoalIds.map((id, index) => {
        const goal = goals.find(g => g.id === id);
        return goal ? { ...goal, priority: index } : null;
      }).filter(Boolean) as SavingsGoal[];
      
      setGoals(reorderedGoals);

      // Update each goal's priority in the database
      const updates = orderedGoalIds.map((id, index) => 
        supabase
          .from('savings_goals')
          .update({ priority: index })
          .eq('id', id)
      );

      await Promise.all(updates);
    } catch (err: any) {
      setError(err);
      // Revert on error
      await fetchGoals();
      throw err;
    }
  };

  const addContribution = async (contributionData: {
    goal_id: string;
    user_id: string;
    amount: number;
    date: string;
    expense_id?: string | null;
  }) => {
    try {
      const { data, error } = await supabase
        .from('goal_contributions')
        .insert(contributionData)
        .select()
        .single();

      if (error) throw error;
      await fetchGoals();
      return data;
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  const deleteContribution = async (id: string) => {
    try {
      const { error } = await supabase.from('goal_contributions').delete().eq('id', id);

      if (error) throw error;
      await fetchGoals();
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  // Allocate unallocated savings to a specific goal
  const allocateSavings = async (unallocatedId: string, goalId: string, userId: string) => {
    try {
      const unallocated = unallocatedSavings.find(u => u.id === unallocatedId);
      if (!unallocated) throw new Error('Unallocated savings not found');

      // Create contribution to the goal
      await addContribution({
        goal_id: goalId,
        user_id: userId,
        amount: unallocated.amount,
        date: unallocated.date,
        expense_id: unallocated.expense_id,
      });

      // Mark as allocated
      const { error } = await supabase
        .from('unallocated_savings')
        .update({
          allocated_at: new Date().toISOString(),
          allocated_to_goal_id: goalId,
        })
        .eq('id', unallocatedId);

      if (error) throw error;
      await fetchUnallocatedSavings();
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  // Add unallocated savings entry
  const addUnallocatedSavings = async (data: {
    household_id: string;
    expense_id?: string | null;
    amount: number;
    date: string;
    description?: string | null;
  }) => {
    try {
      const { error } = await supabase
        .from('unallocated_savings')
        .insert(data);

      if (error) throw error;
      await fetchUnallocatedSavings();
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  // Process a savings deposit and auto-fill goals by priority
  const processSavingsDeposit = useCallback(async (
    accountId: string,
    amount: number,
    userId: string,
    expenseId?: string | null,
    date?: string
  ) => {
    if (!householdId) return { allocated: 0, remaining: amount };

    try {
      // Get goals linked to this account, ordered by priority
      const linkedGoals = goals
        .filter(g => g.account_id === accountId)
        .sort((a, b) => (a.priority || 0) - (b.priority || 0));

      if (linkedGoals.length === 0) {
        // No goals linked to this account, add to unallocated
        await addUnallocatedSavings({
          household_id: householdId,
          expense_id: expenseId || null,
          amount,
          date: date || new Date().toISOString().split('T')[0],
          description: 'Auto-detected savings deposit',
        });
        return { allocated: 0, remaining: amount };
      }

      let remainingAmount = amount;
      let totalAllocated = 0;
      const depositDate = date || new Date().toISOString().split('T')[0];

      for (const goal of linkedGoals) {
        if (remainingAmount <= 0) break;

        const progress = getGoalProgress(goal);
        const remaining = goal.target_amount - progress.current;

        if (remaining <= 0) continue; // Goal already filled

        const contribution = Math.min(remainingAmount, remaining);
        
        await addContribution({
          goal_id: goal.id,
          user_id: userId,
          amount: contribution,
          date: depositDate,
          expense_id: expenseId || null,
        });

        remainingAmount -= contribution;
        totalAllocated += contribution;
      }

      // If there's remaining amount after filling all goals, add to unallocated
      if (remainingAmount > 0) {
        await addUnallocatedSavings({
          household_id: householdId,
          expense_id: expenseId || null,
          amount: remainingAmount,
          date: depositDate,
          description: 'Overflow from auto-fill',
        });
      }

      return { allocated: totalAllocated, remaining: remainingAmount };
    } catch (err: any) {
      setError(err);
      throw err;
    }
  }, [householdId, goals, contributions]);

  // Get goals linked to a specific account
  const getAccountLinkedGoals = useCallback((accountId: string) => {
    return goals
      .filter(g => g.account_id === accountId)
      .sort((a, b) => (a.priority || 0) - (b.priority || 0));
  }, [goals]);

  const getGoalProgress = useCallback((goal: SavingsGoal): { current: number; percentage: number; remaining: number } => {
    const goalContributions = contributions[goal.id] || [];
    const current = goalContributions.reduce((sum, contrib) => sum + contrib.amount, 0);
    const percentage = Math.min((current / goal.target_amount) * 100, 100);
    const remaining = Math.max(goal.target_amount - current, 0);
    return { current, percentage, remaining };
  }, [contributions]);

  // Get summary stats
  const getSummaryStats = useCallback(() => {
    let totalSaved = 0;
    let totalTarget = 0;
    let completedGoals = 0;

    goals.forEach(goal => {
      const progress = getGoalProgress(goal);
      totalSaved += progress.current;
      totalTarget += goal.target_amount;
      if (progress.percentage >= 100) {
        completedGoals++;
      }
    });

    const unallocatedTotal = unallocatedSavings.reduce((sum, u) => sum + u.amount, 0);

    return {
      totalSaved,
      totalTarget,
      completedGoals,
      activeGoals: goals.length - completedGoals,
      unallocatedTotal,
      overallPercentage: totalTarget > 0 ? (totalSaved / totalTarget) * 100 : 0,
    };
  }, [goals, contributions, unallocatedSavings]);

  return {
    goals,
    contributions,
    unallocatedSavings,
    loading,
    error,
    createGoal,
    updateGoal,
    deleteGoal,
    reorderGoals,
    addContribution,
    deleteContribution,
    allocateSavings,
    addUnallocatedSavings,
    processSavingsDeposit,
    getAccountLinkedGoals,
    getGoalProgress,
    getSummaryStats,
    refetch: fetchGoals,
    refetchUnallocated: fetchUnallocatedSavings,
  };
}
