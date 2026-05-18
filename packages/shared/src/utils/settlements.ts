interface CalculateSharedSettlementsParams {
  supabase: any;
  householdId: string;
  categoryIds: string[];
  memberIds: string[];
  splitsByCategory: Record<string, Record<string, number>>;
  dateFrom?: string;
  dateTo?: string;
  dateRanges?: Array<{ from?: string; to?: string }>;
}

export type SharedSplitMap = Record<string, Record<string, number>>;

export const buildEvenSplit = (memberIds: string[]): Record<string, number> => {
  if (memberIds.length === 0) return {};
  const base = 100 / memberIds.length;
  const split: Record<string, number> = {};
  let runningTotal = 0;

  memberIds.forEach((memberId, index) => {
    if (index === memberIds.length - 1) {
      split[memberId] = parseFloat((100 - runningTotal).toFixed(2));
    } else {
      const rounded = parseFloat(base.toFixed(2));
      split[memberId] = rounded;
      runningTotal += rounded;
    }
  });

  return split;
};

const normalizeSplit = (percentages: number[], memberIds: string[]): Record<string, number> => {
  if (memberIds.length === 0) return {};
  const total = percentages.reduce((sum, val) => sum + val, 0);

  // If totals are zero, fall back to an even split
  if (total === 0) {
    return buildEvenSplit(memberIds);
  }

  const normalizedRaw = percentages.map((pct) => (pct / total) * 100);
  const normalized: Record<string, number> = {};
  let running = 0;

  normalizedRaw.forEach((pct, index) => {
    if (index === memberIds.length - 1) {
      normalized[memberIds[index]] = parseFloat((100 - running).toFixed(2));
    } else {
      const rounded = parseFloat(pct.toFixed(2));
      normalized[memberIds[index]] = rounded;
      running += rounded;
    }
  });

  return normalized;
};

const getCategorySplit = (
  categoryId: string,
  memberIds: string[],
  splitsByCategory: SharedSplitMap
): Record<string, number> => {
  const provided = splitsByCategory[categoryId] || {};
  const percentages = memberIds.map((id) => provided[id] ?? 100 / (memberIds.length || 1));
  return normalizeSplit(percentages, memberIds);
};

export async function calculateSharedSettlements({
  supabase,
  householdId,
  categoryIds,
  memberIds,
  splitsByCategory,
  dateFrom,
  dateTo,
  dateRanges,
}: CalculateSharedSettlementsParams) {
  if (!householdId || memberIds.length === 0 || categoryIds.length === 0) {
    return {
      settlements: [],
      balances: {} as Record<string, number>,
      totalSharedSpend: 0,
      categoryTotals: {} as Record<string, number>,
    };
  }

  let query = supabase
    .from('expenses')
    .select('id, amount, payer_id, category_id, date')
    .eq('household_id', householdId)
    .in('category_id', categoryIds);

  const ranges = (dateRanges || []).filter((r) => r.from || r.to);
  if (ranges.length > 0) {
    const parts = ranges.map((range) => {
      const from = range.from;
      const to = range.to;
      if (from && to) return `and(date.gte.${from},date.lte.${to})`;
      if (from) return `date.gte.${from}`;
      if (to) return `date.lte.${to}`;
      return '';
    });
    const clause = parts.filter(Boolean).join(',');
    if (clause) {
      query = query.or(clause);
    }
  } else {
    if (dateFrom) {
      query = query.gte('date', dateFrom);
    }
    if (dateTo) {
      query = query.lte('date', dateTo);
    }
  }

  const { data: expenses, error: expensesError } = await query;
  if (expensesError) throw expensesError;

  const balances: Record<string, number> = {};
  memberIds.forEach((id) => (balances[id] = 0));
  const categoryTotals: Record<string, number> = {};
  let totalSharedSpend = 0;

  expenses?.forEach((expense: any) => {
    // Use absolute value for settlement calculations - negative amounts represent credits/refunds
    // but for settlement purposes, we always work with positive expense amounts
    const rawAmount = Number(expense.amount) || 0;
    const amount = Math.abs(rawAmount);
    const split = getCategorySplit(expense.category_id, memberIds, splitsByCategory);
    totalSharedSpend += amount;
    categoryTotals[expense.category_id] = (categoryTotals[expense.category_id] ?? 0) + amount;

    // payer paid full amount
    balances[expense.payer_id] = (balances[expense.payer_id] ?? 0) + amount;

    // each member owes their split
    memberIds.forEach((memberId) => {
      const pct = split[memberId] ?? 0;
      const owed = (amount * pct) / 100;
      balances[memberId] = (balances[memberId] ?? 0) - owed;
    });
  });

  const settlements: Array<{ from: string; to: string; amount: number }> = [];

  const positiveBalances = Object.entries(balances)
    .filter(([, balance]) => balance > 0.01)
    .sort(([, a], [, b]) => b - a);

  const negativeBalances = Object.entries(balances)
    .filter(([, balance]) => balance < -0.01)
    .sort(([, a], [, b]) => a - b);

  let posIndex = 0;
  let negIndex = 0;

  while (posIndex < positiveBalances.length && negIndex < negativeBalances.length) {
    const [debtorId, debtorBalance] = negativeBalances[negIndex];
    const [creditorId, creditorBalance] = positiveBalances[posIndex];

    const amount = Math.min(Math.abs(debtorBalance), creditorBalance);

    settlements.push({
      from: debtorId,
      to: creditorId,
      amount: parseFloat(amount.toFixed(2)),
    });

    negativeBalances[negIndex][1] = debtorBalance + amount;
    positiveBalances[posIndex][1] = creditorBalance - amount;

    if (Math.abs(negativeBalances[negIndex][1]) < 0.01) {
      negIndex++;
    }
    if (positiveBalances[posIndex][1] < 0.01) {
      posIndex++;
    }
  }

  return {
    settlements,
    balances,
    totalSharedSpend: parseFloat(totalSharedSpend.toFixed(2)),
    categoryTotals,
  };
}

