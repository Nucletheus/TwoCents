/**
 * Utility functions for processing savings-related transactions after import
 */

/**
 * Check if a category is marked as a savings category
 * Uses the is_savings_category flag from the database
 */
export function isSavingsCategory(category: { is_savings_category?: boolean; name?: string }): boolean {
  // Prefer the explicit flag
  if (category.is_savings_category !== undefined) {
    return category.is_savings_category;
  }
  // Fallback to name-based detection for backwards compatibility
  if (category.name) {
    const lowerName = category.name.toLowerCase();
    return ['savings', 'emergency fund', 'savings contribution', 'investment contribution', 'retirement contribution']
      .some(pattern => lowerName.includes(pattern.toLowerCase()));
  }
  return false;
}

/**
 * Process imported expenses to detect savings deposits and create contributions/unallocated entries
 * 
 * @param supabase - Supabase client
 * @param householdId - The household ID
 * @param userId - The user who imported the transactions
 * @param importedExpenses - Array of imported expenses (must include id, account_id, amount, category_id, date)
 * @param categories - Array of all categories (to check is_savings_category flag)
 */
export async function processSavingsFromImport(
  supabase: any,
  householdId: string,
  userId: string,
  importedExpenses: Array<{
    id?: string;
    account_id?: string | null;
    amount: number;
    category_id?: string | null;
    date: string;
    description?: string | null;
  }>,
  categories: Array<{ id: string; name: string; is_savings_category?: boolean }>
): Promise<{
  accountLinkedCount: number;
  unallocatedCount: number;
  errors: string[];
}> {
  const result = {
    accountLinkedCount: 0,
    unallocatedCount: 0,
    errors: [] as string[],
  };

  // Build category lookup map
  const categoryMap = new Map(categories.map(c => [c.id, c]));

  // Get savings accounts in this household
  const { data: savingsAccounts, error: accountsError } = await supabase
    .from('accounts')
    .select('id, name')
    .eq('household_id', householdId)
    .eq('type', 'savings');

  if (accountsError) {
    result.errors.push(`Failed to fetch savings accounts: ${accountsError.message}`);
    return result;
  }

  const savingsAccountIds = new Set(savingsAccounts?.map((a: any) => a.id) || []);

  // Get savings goals linked to accounts, ordered by priority
  const { data: goalsData, error: goalsError } = await supabase
    .from('savings_goals')
    .select('id, account_id, target_amount, priority')
    .eq('household_id', householdId)
    .not('account_id', 'is', null)
    .order('priority', { ascending: true });

  if (goalsError) {
    result.errors.push(`Failed to fetch savings goals: ${goalsError.message}`);
    return result;
  }

  // Build map of account_id -> goals (sorted by priority)
  const goalsByAccount = new Map<string, typeof goalsData>();
  goalsData?.forEach((goal: any) => {
    if (!goalsByAccount.has(goal.account_id)) {
      goalsByAccount.set(goal.account_id, []);
    }
    goalsByAccount.get(goal.account_id)!.push(goal);
  });

  // Get existing contributions for these goals to calculate remaining amounts
  const goalIds = goalsData?.map((g: any) => g.id) || [];
  let contributionsByGoal = new Map<string, number>();
  
  if (goalIds.length > 0) {
    const { data: contributions, error: contribError } = await supabase
      .from('goal_contributions')
      .select('goal_id, amount')
      .in('goal_id', goalIds);

    if (!contribError && contributions) {
      contributions.forEach((c: any) => {
        const current = contributionsByGoal.get(c.goal_id) || 0;
        contributionsByGoal.set(c.goal_id, current + c.amount);
      });
    }
  }

  // Process each imported expense
  for (const expense of importedExpenses) {
    // Skip if amount is 0 or negative (not a deposit)
    if (expense.amount <= 0) continue;

    const isToSavingsAccount = expense.account_id && savingsAccountIds.has(expense.account_id);
    const category = expense.category_id ? categoryMap.get(expense.category_id) : null;
    const isCategorizedAsSavings = category && isSavingsCategory(category);

    // Case 1: Transaction to a savings account linked to goals
    if (isToSavingsAccount && goalsByAccount.has(expense.account_id!)) {
      const linkedGoals = goalsByAccount.get(expense.account_id!)!;
      let remainingAmount = expense.amount;

      for (const goal of linkedGoals) {
        if (remainingAmount <= 0) break;

        const currentContributed = contributionsByGoal.get(goal.id) || 0;
        const remaining = goal.target_amount - currentContributed;

        if (remaining <= 0) continue; // Goal already filled

        const contribution = Math.min(remainingAmount, remaining);

        // Create contribution
        const { error: contribError } = await supabase
          .from('goal_contributions')
          .insert({
            goal_id: goal.id,
            user_id: userId,
            amount: contribution,
            date: expense.date,
            expense_id: expense.id || null,
          });

        if (contribError) {
          result.errors.push(`Failed to create contribution: ${contribError.message}`);
        } else {
          result.accountLinkedCount++;
          remainingAmount -= contribution;
          // Update local tracking
          contributionsByGoal.set(goal.id, currentContributed + contribution);
        }
      }

      // If there's remaining amount after filling all linked goals, add to unallocated
      if (remainingAmount > 0) {
        const { error: unallocError } = await supabase
          .from('unallocated_savings')
          .insert({
            household_id: householdId,
            expense_id: expense.id || null,
            amount: remainingAmount,
            date: expense.date,
            description: expense.description || 'Overflow from account-linked deposit',
          });

        if (unallocError) {
          result.errors.push(`Failed to create unallocated entry: ${unallocError.message}`);
        } else {
          result.unallocatedCount++;
        }
      }
    }
    // Case 2: Transaction to a savings account NOT linked to any goal
    else if (isToSavingsAccount) {
      // Add to unallocated savings
      const { error: unallocError } = await supabase
        .from('unallocated_savings')
        .insert({
          household_id: householdId,
          expense_id: expense.id || null,
          amount: expense.amount,
          date: expense.date,
          description: expense.description || 'Deposit to unlinked savings account',
        });

      if (unallocError) {
        result.errors.push(`Failed to create unallocated entry: ${unallocError.message}`);
      } else {
        result.unallocatedCount++;
      }
    }
    // Case 3: Transaction categorized as savings (but not to a savings account)
    else if (isCategorizedAsSavings) {
      // Add to unallocated savings
      const { error: unallocError } = await supabase
        .from('unallocated_savings')
        .insert({
          household_id: householdId,
          expense_id: expense.id || null,
          amount: expense.amount,
          date: expense.date,
          description: expense.description || 'Categorized as savings',
        });

      if (unallocError) {
        result.errors.push(`Failed to create unallocated entry: ${unallocError.message}`);
      } else {
        result.unallocatedCount++;
      }
    }
  }

  return result;
}

/**
 * Process a single expense that was updated to check if it should be tracked as savings
 * Call this when an expense's category is changed to a savings category
 * 
 * @param supabase - Supabase client
 * @param expense - The expense that was updated
 * @param category - The category it was changed to (must have is_savings_category field)
 */
export async function processExpenseForSavings(
  supabase: any,
  expense: {
    id: string;
    household_id: string;
    account_id?: string | null;
    amount: number;
    date: string;
    description?: string | null;
  },
  category: { id: string; name: string; is_savings_category?: boolean }
): Promise<{ success: boolean; error?: string }> {
  // Only process if the category is marked as savings
  if (!isSavingsCategory(category)) {
    return { success: true }; // Not a savings category, nothing to do
  }

  // Skip if amount is 0 or negative
  if (expense.amount <= 0) {
    return { success: true };
  }

  // Check if this expense already has an unallocated savings entry
  const { data: existing, error: checkError } = await supabase
    .from('unallocated_savings')
    .select('id')
    .eq('expense_id', expense.id)
    .is('allocated_at', null)
    .single();

  if (existing) {
    // Already tracked, don't duplicate
    return { success: true };
  }

  // Check if this expense is already contributed to a goal
  const { data: existingContrib, error: contribCheckError } = await supabase
    .from('goal_contributions')
    .select('id')
    .eq('expense_id', expense.id)
    .single();

  if (existingContrib) {
    // Already contributed, don't duplicate
    return { success: true };
  }

  // Add to unallocated savings
  const { error: insertError } = await supabase
    .from('unallocated_savings')
    .insert({
      household_id: expense.household_id,
      expense_id: expense.id,
      amount: expense.amount,
      date: expense.date,
      description: expense.description || `Categorized as ${category.name}`,
    });

  if (insertError) {
    return { success: false, error: insertError.message };
  }

  return { success: true };
}

