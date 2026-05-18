'use client';

import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
import { useSavingsGoals, formatCurrency } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import Button from '../../components/ui/Button';
import { RefreshCw, Sparkles } from 'lucide-react';

interface AIInsightsCardProps {
  householdId: string;
  timeframe: 'month' | 'year' | 'all' | 'custom';
  timeframeKey: string;
}

export default function AIInsightsCard({ householdId, timeframe, timeframeKey }: AIInsightsCardProps) {
  const supabase = createClient();
  const { goals, contributions } = useSavingsGoals(supabase, householdId);
  const [insights, setInsights] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [generating, setGenerating] = useState(false);

  useEffect(() => {
    fetchInsights();
  }, [householdId, timeframe, timeframeKey]);

  const fetchInsights = async () => {
    if (!householdId) return;

    try {
      const { data, error } = await supabase
        .from('analytics_insights')
        .select('insights_text, generated_at')
        .eq('household_id', householdId)
        .eq('timeframe_type', timeframe)
        .eq('timeframe_key', timeframeKey)
        .maybeSingle();

      if (error && error.code !== 'PGRST116') {
        // PGRST116 is "not found" which is fine
        console.error('Error fetching insights:', error);
        return;
      }

      if (data) {
        setInsights(data.insights_text);
      } else {
        setInsights(null);
      }
    } catch (error) {
      console.error('Error fetching insights:', error);
    }
  };

  const generateInsights = async () => {
    if (!householdId) return;

    setGenerating(true);
    try {
      // Fetch expenses for the timeframe to generate insights
      const { data: expensesData } = await supabase
        .from('expenses')
        .select(
          `
          *,
          categories (
            id,
            name,
            group_name
          )
        `
        )
        .eq('household_id', householdId)
        .order('date', { ascending: false })
        .limit(1000);

      // Fetch budgets
      const { data: mainBudgetSet } = await supabase
        .from('budget_sets')
        .select('id')
        .eq('household_id', householdId)
        .eq('is_main', true)
        .maybeSingle();

      let budgetsData: any[] = [];
      if (mainBudgetSet?.id) {
        const { data } = await supabase
          .from('budgets')
          .select(
            `
            *,
            categories (
              id,
              name,
              group_name
            )
          `
          )
          .eq('budget_set_id', mainBudgetSet.id);
        if (data) budgetsData = data;
      }

      // Fetch categories for group_name filtering
      const { data: categoriesData } = await supabase
        .from('categories')
        .select('id, name, group_name')
        .eq('household_id', householdId)
        .neq('is_hidden', true);

      // Generate mock insights based on data
      const mockInsights = generateMockInsights(
        expensesData || [],
        budgetsData,
        categoriesData || [],
        goals,
        contributions
      );

      // Get current user
      const {
        data: { user },
      } = await supabase.auth.getUser();

      // Store insights in database
      const { error: upsertError } = await supabase
        .from('analytics_insights')
        .upsert(
          {
            household_id: householdId,
            timeframe_type: timeframe,
            timeframe_key: timeframeKey,
            insights_text: mockInsights,
            created_by: user?.id || null,
          },
          { onConflict: 'household_id,timeframe_type,timeframe_key' }
        );

      if (upsertError) {
        console.error('Error storing insights:', upsertError);
      } else {
        setInsights(mockInsights);
      }
    } catch (error) {
      console.error('Error generating insights:', error);
    } finally {
      setGenerating(false);
    }
  };

  // Helper to check if category should be excluded from spending analysis
  const isExcludedFromSpending = (groupName: string | null | undefined): boolean => {
    if (!groupName) return false;
    if (groupName === 'Savings & Investments') return true;
    if (groupName === 'Transfers') return true;
    if (groupName.startsWith('Income')) return true;
    return false;
  };

  const generateMockInsights = (
    expenses: any[],
    budgets: any[],
    categories: any[],
    savingsGoals: any[],
    goalContributions: Record<string, any[]>
  ): string => {
    if (expenses.length === 0) {
      return '**Key Trends:**\n- No spending data available for this timeframe.\n\n**Recommendations:**\n- Start tracking expenses to get insights.';
    }

    // Calculate total spending (use absolute values for display - expenses are stored as negative)
    const totalSpent = Math.abs(expenses.reduce((sum, e) => sum + Number(e.amount || 0), 0));
    const avgExpense = expenses.length > 0 ? totalSpent / expenses.length : 0;

    // Group by category (excluding savings/transfers/income)
    const categorySpending: Record<string, { amount: number; groupName: string | null }> = {};
    expenses.forEach((expense) => {
      const categoryName = expense.categories?.name || 'Uncategorized';
      const groupName = expense.categories?.group_name || null;
      if (!isExcludedFromSpending(groupName)) {
        if (!categorySpending[categoryName]) {
          categorySpending[categoryName] = { amount: 0, groupName };
        }
        // Use absolute value for display (expenses are stored as negative)
        categorySpending[categoryName].amount += Math.abs(Number(expense.amount || 0));
      }
    });

    // Find top spending categories (excluding savings)
    const topCategories = Object.entries(categorySpending)
      .sort(([, a], [, b]) => b.amount - a.amount)
      .slice(0, 5)
      .map(([name, data]) => ({ name, amount: data.amount, groupName: data.groupName }));

    // Calculate budget vs actual
    const totalBudget = budgets.reduce((sum, b) => sum + Number(b.amount || 0), 0);
    const budgetComparison = totalBudget > 0 ? ((totalSpent / totalBudget) * 100).toFixed(1) : null;

    // Find recent spending trends (last 7 days vs previous 7 days if available)
    const now = new Date();
    const sevenDaysAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
    const fourteenDaysAgo = new Date(now.getTime() - 14 * 24 * 60 * 60 * 1000);

    // Use absolute values for display (expenses are stored as negative)
    const recentSpending = Math.abs(expenses
      .filter((e) => new Date(e.date) >= sevenDaysAgo)
      .reduce((sum, e) => sum + Number(e.amount || 0), 0));
    const previousSpending = Math.abs(expenses
      .filter((e) => {
        const date = new Date(e.date);
        return date >= fourteenDaysAgo && date < sevenDaysAgo;
      })
      .reduce((sum, e) => sum + Number(e.amount || 0), 0));

    const spendingChange =
      previousSpending > 0 ? (((recentSpending - previousSpending) / previousSpending) * 100).toFixed(1) : null;

    // Build insights text
    let insights = '**Key Trends:**\n';
    if (spendingChange) {
      const change = parseFloat(spendingChange);
      if (change > 0) {
        insights += `- Spending increased ${Math.abs(change)}% compared to the previous week.\n`;
      } else if (change < 0) {
        insights += `- Spending decreased ${Math.abs(change)}% compared to the previous week.\n`;
      } else {
        insights += `- Spending remained stable compared to the previous week.\n`;
      }
    }
    insights += `- Average transaction amount: $${avgExpense.toFixed(2)}\n`;
    if (topCategories.length > 0) {
      insights += `- Top spending category: ${topCategories[0].name} (${((topCategories[0].amount / totalSpent) * 100).toFixed(1)}% of total)\n`;
    }

    insights += '\n**Anomalies:**\n';
    if (expenses.length > 0) {
      // Compare absolute values to find largest expense
      const largestExpense = expenses.reduce((max, e) =>
        Math.abs(Number(e.amount || 0)) > Math.abs(Number(max.amount || 0)) ? e : max
      );
      const largestAmount = Math.abs(Number(largestExpense.amount || 0));
      if (largestAmount > avgExpense * 3) {
        insights += `- Unusually large expense detected: $${largestAmount.toFixed(2)} in ${largestExpense.categories?.name || 'Uncategorized'} category\n`;
      } else {
        insights += `- No significant anomalies detected in spending patterns.\n`;
      }
    }

    insights += '\n**Recommendations:**\n';
    if (budgetComparison) {
      const budgetPercent = parseFloat(budgetComparison);
      if (budgetPercent > 100) {
        insights += `- You've exceeded your budget by ${(budgetPercent - 100).toFixed(1)}%. Consider reducing spending in high-cost categories.\n`;
      } else if (budgetPercent > 80) {
        insights += `- You're at ${budgetComparison}% of your budget. Monitor spending closely to stay within budget.\n`;
      } else {
        insights += `- You're at ${budgetComparison}% of your budget. Good progress!\n`;
      }
    }
    if (topCategories.length > 0 && topCategories[0].amount / totalSpent > 0.4) {
      insights += `- Consider diversifying spending. ${topCategories[0].name} accounts for a large portion of expenses.\n`;
    }
    if (expenses.length < 10) {
      insights += `- Limited data available. More transactions will provide better insights.\n`;
    }

    // Calculate savings goal what-if scenarios
    const whatIfScenarios = calculateWhatIfScenarios(
      expenses,
      topCategories,
      savingsGoals,
      goalContributions,
      categories
    );
    if (whatIfScenarios.length > 0) {
      insights += '\n**What-If Scenarios:**\n';
      whatIfScenarios.forEach((scenario) => {
        insights += scenario + '\n';
      });
    }

    return insights;
  };

  const calculateWhatIfScenarios = (
    expenses: any[],
    topCategories: Array<{ name: string; amount: number }>,
    savingsGoals: any[],
    goalContributions: Record<string, any[]>,
    categories: any[]
  ): string[] => {
    const scenarios: string[] = [];

    // Get active savings goals (not completed)
    const activeGoals = savingsGoals.filter((goal) => {
      const goalContribs = goalContributions[goal.id] || [];
      const current = goalContribs.reduce((sum: number, contrib: any) => sum + contrib.amount, 0);
      return current < goal.target_amount;
    });

    if (activeGoals.length === 0 || topCategories.length === 0) {
      return scenarios;
    }

    // Get the highest priority active goal
    const primaryGoal = activeGoals.sort((a, b) => (a.priority || 0) - (b.priority || 0))[0];
    const goalContribs = goalContributions[primaryGoal.id] || [];
    const currentAmount = goalContribs.reduce((sum: number, contrib: any) => sum + contrib.amount, 0);
    const remaining = primaryGoal.target_amount - currentAmount;

    // Calculate current monthly savings rate
    const savingsCategoryGroup = 'Savings & Investments';
    const savingsExpenses = expenses.filter((e) => {
      const groupName = e?.categories?.group_name;
      return groupName === savingsCategoryGroup && Number(e.amount || 0) > 0;
    });

    // Group savings by month to calculate average monthly savings
    const monthlySavings: number[] = [];
    const savingsByMonth: Record<string, number> = {};
    savingsExpenses.forEach((expense) => {
      const month = new Date(expense.date).toLocaleDateString('en-US', {
        month: 'short',
        year: 'numeric',
      });
      savingsByMonth[month] = (savingsByMonth[month] || 0) + Number(expense.amount || 0);
    });
    monthlySavings.push(...Object.values(savingsByMonth));

    const avgMonthlySavings =
      monthlySavings.length > 0
        ? monthlySavings.slice(-3).reduce((sum, val) => sum + val, 0) / Math.min(3, monthlySavings.length)
        : 0;

    // Calculate current projected completion date
    const currentMonthsToGoal = avgMonthlySavings > 0 ? Math.ceil(remaining / avgMonthlySavings) : null;
    if (currentMonthsToGoal === null || currentMonthsToGoal <= 0) {
      return scenarios;
    }

    const currentDate = new Date();
    const currentCompletionDate = new Date(currentDate);
    currentCompletionDate.setMonth(currentDate.getMonth() + currentMonthsToGoal);

    // Calculate monthly spending in top categories (for reduction scenarios)
    const monthlyCategorySpending: Record<string, number> = {};
    expenses.forEach((expense) => {
      const categoryName = expense.categories?.name || 'Uncategorized';
      const groupName = expense.categories?.group_name || null;
      if (!isExcludedFromSpending(groupName) && topCategories.some((c) => c.name === categoryName)) {
        const month = new Date(expense.date).toLocaleDateString('en-US', {
          month: 'short',
          year: 'numeric',
        });
        const key = `${categoryName}_${month}`;
        monthlyCategorySpending[key] = (monthlyCategorySpending[key] || 0) + Number(expense.amount || 0);
      }
    });

    // Calculate average monthly spending per top category
    const categoryMonthlyAverages: Record<string, number> = {};
    topCategories.forEach((cat) => {
      const categoryMonths = Object.keys(monthlyCategorySpending)
        .filter((key) => key.startsWith(cat.name + '_'))
        .map((key) => monthlyCategorySpending[key]);
      if (categoryMonths.length > 0) {
        categoryMonthlyAverages[cat.name] =
          categoryMonths.reduce((sum, val) => sum + val, 0) / categoryMonths.length;
      }
    });

    // Generate what-if scenarios for top 2-3 categories
    const scenariosToGenerate = Math.min(3, topCategories.length);
    for (let i = 0; i < scenariosToGenerate; i++) {
      const category = topCategories[i];
      const avgMonthlySpending = categoryMonthlyAverages[category.name] || 0;

      if (avgMonthlySpending <= 0) continue;

      // Try different reduction percentages: 10%, 20%, 30%
      for (const reductionPercent of [10, 20, 30]) {
        const monthlySavings = (avgMonthlySpending * reductionPercent) / 100;
        const newMonthlySavings = avgMonthlySavings + monthlySavings;
        const newMonthsToGoal = Math.ceil(remaining / newMonthlySavings);

        if (newMonthsToGoal < currentMonthsToGoal) {
          const newCompletionDate = new Date(currentDate);
          newCompletionDate.setMonth(currentDate.getMonth() + newMonthsToGoal);

          const monthsSaved = currentMonthsToGoal - newMonthsToGoal;
          const dateDiff = formatDateDifference(currentCompletionDate, newCompletionDate);

          scenarios.push(
            `- If you reduce ${category.name} spending by ${reductionPercent}% (saving ${formatCurrency(monthlySavings)}/month), you could reach "${primaryGoal.name}" ${dateDiff} earlier (${formatDate(newCompletionDate)} vs ${formatDate(currentCompletionDate)})`
          );

          // Only show one scenario per category (the most impactful)
          break;
        }
      }
    }

    // If multiple categories, show combined scenario
    if (topCategories.length >= 2) {
      const topTwoCategories = topCategories.slice(0, 2);
      let combinedMonthlySavings = 0;
      let categoryNames: string[] = [];

      topTwoCategories.forEach((cat) => {
        const avgMonthlySpending = categoryMonthlyAverages[cat.name] || 0;
        if (avgMonthlySpending > 0) {
          // 15% reduction for combined scenario
          combinedMonthlySavings += (avgMonthlySpending * 15) / 100;
          categoryNames.push(cat.name);
        }
      });

      if (combinedMonthlySavings > 0) {
        const newMonthlySavings = avgMonthlySavings + combinedMonthlySavings;
        const newMonthsToGoal = Math.ceil(remaining / newMonthlySavings);

        if (newMonthsToGoal < currentMonthsToGoal) {
          const newCompletionDate = new Date(currentDate);
          newCompletionDate.setMonth(currentDate.getMonth() + newMonthsToGoal);

          const monthsSaved = currentMonthsToGoal - newMonthsToGoal;
          const dateDiff = formatDateDifference(currentCompletionDate, newCompletionDate);

          scenarios.push(
            `- If you reduce spending in ${categoryNames.join(' and ')} by 15% each (saving ${formatCurrency(combinedMonthlySavings)}/month combined), you could reach "${primaryGoal.name}" ${dateDiff} earlier (${formatDate(newCompletionDate)} vs ${formatDate(currentCompletionDate)})`
          );
        }
      }
    }

    return scenarios.slice(0, 4); // Limit to 4 scenarios to keep it readable
  };

  const formatDate = (date: Date): string => {
    return date.toLocaleDateString('en-US', { month: 'short', year: 'numeric' });
  };

  const formatDateDifference = (date1: Date, date2: Date): string => {
    const monthsDiff = (date1.getFullYear() - date2.getFullYear()) * 12 + (date1.getMonth() - date2.getMonth());
    if (monthsDiff === 1) return '1 month';
    if (monthsDiff > 1) return `${monthsDiff} months`;
    const daysDiff = Math.floor((date1.getTime() - date2.getTime()) / (1000 * 60 * 60 * 24));
    if (daysDiff === 1) return '1 day';
    if (daysDiff > 1) return `${daysDiff} days`;
    return 'sooner';
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-accent" />
            <CardTitle>AI Insights</CardTitle>
          </div>
          <Button
            variant="secondary"
            size="sm"
            onClick={generateInsights}
            disabled={generating}
            className="flex items-center gap-2"
          >
            <RefreshCw className={`h-4 w-4 ${generating ? 'animate-spin' : ''}`} />
            {generating ? 'Generating...' : 'Generate Insights'}
          </Button>
        </div>
        <CardDescription>AI-driven analysis of your spending patterns and trends</CardDescription>
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="text-muted-foreground">Loading insights...</div>
        ) : insights ? (
          <div className="whitespace-pre-wrap text-sm leading-relaxed">{insights}</div>
        ) : (
          <div className="text-center py-8 text-muted-foreground">
            <p>No insights available for this timeframe.</p>
            <p className="text-xs mt-2">Click "Generate Insights" to create AI-driven analysis.</p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

