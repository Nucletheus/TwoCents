'use client';

import { useMemo } from 'react';
import { createClient } from '@/lib/supabase/client';
import { useSavingsGoals, formatCurrency } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  BarChart,
  Bar,
} from 'recharts';
import { Target } from 'lucide-react';

interface SavingsGoalChartProps {
  householdId: string;
  expenses: any[];
  budgets: any[];
  categories: any[];
}

export default function SavingsGoalChart({ householdId, expenses, budgets, categories }: SavingsGoalChartProps) {
  const supabase = createClient();
  const { goals, contributions, loading } = useSavingsGoals(supabase, householdId);

  // Filter to active goals (not completed)
  const activeGoals = useMemo(() => {
    return goals.filter((goal) => {
      const goalContributions = contributions[goal.id] || [];
      const current = goalContributions.reduce((sum, contrib) => sum + contrib.amount, 0);
      return current < goal.target_amount;
    });
  }, [goals, contributions]);

  // Get savings category group
  const savingsCategoryGroup = 'Savings & Investments';

  // Calculate budgeted monthly savings
  const budgetedMonthlySavings = useMemo(() => {
    return budgets
      .filter((b) => {
        const category = categories.find((c) => c.id === b.category_id);
        return category?.group_name === savingsCategoryGroup;
      })
      .reduce((sum, b) => sum + Number(b.amount || 0), 0);
  }, [budgets, categories]);

  // Calculate actual savings by month
  const actualSavingsByMonth = useMemo(() => {
    const monthly: Record<string, number> = {};
    expenses
      .filter((e) => {
        const groupName = e?.categories?.group_name;
        return groupName === savingsCategoryGroup && Number(e.amount || 0) > 0;
      })
      .forEach((expense) => {
        const month = new Date(expense.date).toLocaleDateString('en-US', {
          month: 'short',
          year: 'numeric',
        });
        monthly[month] = (monthly[month] || 0) + Number(expense.amount || 0);
      });
    return monthly;
  }, [expenses]);

  // Calculate goal progress data
  const goalProgressData = useMemo(() => {
    if (activeGoals.length === 0) return [];

    // Get the highest priority active goal
    const primaryGoal = activeGoals.sort((a, b) => (a.priority || 0) - (b.priority || 0))[0];
    const goalContributions = contributions[primaryGoal.id] || [];
    const currentAmount = goalContributions.reduce((sum, contrib) => sum + contrib.amount, 0);
    const remaining = primaryGoal.target_amount - currentAmount;

    // Calculate monthly savings rate (average of last 3 months if available)
    const monthlySavings = Object.values(actualSavingsByMonth);
    const avgMonthlySavings =
      monthlySavings.length > 0
        ? monthlySavings.slice(-3).reduce((sum, val) => sum + val, 0) / Math.min(3, monthlySavings.length)
        : budgetedMonthlySavings || 0;

    // Calculate months needed to reach goal
    const monthsToGoal = avgMonthlySavings > 0 ? Math.ceil(remaining / avgMonthlySavings) : null;

    // Build timeline data
    const timelineData: Array<{
      month: string;
      budgeted: number;
      actual: number;
      projected: number;
      goalTarget: number;
    }> = [];

    // Historical data
    const sortedMonths = Object.keys(actualSavingsByMonth).sort();
    let cumulativeBudgeted = 0;
    let cumulativeActual = 0;

    sortedMonths.forEach((month) => {
      cumulativeBudgeted += budgetedMonthlySavings;
      cumulativeActual += actualSavingsByMonth[month];
      timelineData.push({
        month,
        budgeted: cumulativeBudgeted,
        actual: cumulativeActual,
        projected: cumulativeActual,
        goalTarget: primaryGoal.target_amount,
      });
    });

    // Projected data (next 12 months or until goal is reached)
    const lastMonth = sortedMonths[sortedMonths.length - 1];
    if (lastMonth && avgMonthlySavings > 0) {
      // Parse month string like "Jan 2024" to Date (add day 1)
      const lastDate = new Date(lastMonth + ' 1');
      const projectionMonths = monthsToGoal ? Math.min(monthsToGoal, 12) : 12;

      for (let i = 1; i <= projectionMonths; i++) {
        const forecastDate = new Date(lastDate);
        forecastDate.setMonth(forecastDate.getMonth() + i);
        const monthLabel = forecastDate.toLocaleDateString('en-US', { month: 'short', year: 'numeric' });

        cumulativeBudgeted += budgetedMonthlySavings;
        cumulativeActual += avgMonthlySavings;

        timelineData.push({
          month: monthLabel,
          budgeted: cumulativeBudgeted,
          actual: cumulativeActual,
          projected: cumulativeActual,
          goalTarget: primaryGoal.target_amount,
        });

        // Stop if goal would be reached
        if (cumulativeActual >= primaryGoal.target_amount) {
          break;
        }
      }
    }

    return {
      goal: primaryGoal,
      currentAmount,
      remaining,
      monthsToGoal,
      timelineData,
      avgMonthlySavings,
    };
  }, [activeGoals, contributions, actualSavingsByMonth, budgetedMonthlySavings]);

  if (loading) {
    return (
      <Card>
        <CardContent className="py-8 text-center">
          <div className="text-muted-foreground">Loading savings goals...</div>
        </CardContent>
      </Card>
    );
  }

  if (activeGoals.length === 0) {
    return (
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Target className="h-5 w-5 text-accent" />
            <CardTitle>Savings Goals Progress</CardTitle>
          </div>
          <CardDescription>Track your progress toward savings goals</CardDescription>
        </CardHeader>
        <CardContent className="py-8 text-center">
          <p className="text-muted-foreground">No active savings goals found.</p>
          <p className="text-xs text-muted-foreground mt-2">Create a savings goal to track your progress.</p>
        </CardContent>
      </Card>
    );
  }

  const { goal, currentAmount, remaining, monthsToGoal, timelineData, avgMonthlySavings } = goalProgressData;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Target className="h-5 w-5 text-accent" />
          <CardTitle>Savings Goals Progress</CardTitle>
        </div>
        <CardDescription>
          Tracking progress toward: <strong>{goal.name}</strong> ({formatCurrency(goal.target_amount)})
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          {/* Goal Summary */}
          <div className="grid gap-4 md:grid-cols-3">
            <div>
              <p className="text-sm text-muted-foreground">Current Progress</p>
              <p className="text-2xl font-bold">{formatCurrency(currentAmount)}</p>
              <p className="text-xs text-muted-foreground">
                {((currentAmount / goal.target_amount) * 100).toFixed(1)}% of goal
              </p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Remaining</p>
              <p className="text-2xl font-bold">{formatCurrency(remaining)}</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Projected Completion</p>
              <p className="text-2xl font-bold">
                {monthsToGoal !== null ? `${monthsToGoal} months` : 'N/A'}
              </p>
              <p className="text-xs text-muted-foreground">
                Based on avg savings: {formatCurrency(avgMonthlySavings)}/month
              </p>
            </div>
          </div>

          {/* Progress Chart */}
          {timelineData.length > 0 && (
            <div>
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={timelineData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="month" />
                  <YAxis />
                  <Tooltip
                    formatter={(value: number) => formatCurrency(value)}
                    labelFormatter={(label) => label}
                  />
                  <Legend />
                  <Line
                    type="monotone"
                    dataKey="budgeted"
                    stroke="#10b981"
                    strokeWidth={2}
                    strokeDasharray="5 5"
                    name="Budgeted Savings"
                  />
                  <Line
                    type="monotone"
                    dataKey="actual"
                    stroke="#3b82f6"
                    strokeWidth={2}
                    name="Actual Savings"
                  />
                  <Line
                    type="monotone"
                    dataKey="goalTarget"
                    stroke="#ef4444"
                    strokeWidth={2}
                    strokeDasharray="3 3"
                    name="Goal Target"
                  />
                </LineChart>
              </ResponsiveContainer>
              <p className="text-xs text-muted-foreground mt-2">
                Shows budgeted vs actual savings accumulation. When actual falls behind budgeted, goal completion is
                delayed.
              </p>
            </div>
          )}

          {/* All Active Goals List */}
          {activeGoals.length > 1 && (
            <div>
              <h4 className="text-sm font-medium mb-3">All Active Goals</h4>
              <div className="space-y-2">
                {activeGoals.map((g) => {
                  const goalContributions = contributions[g.id] || [];
                  const current = goalContributions.reduce((sum, contrib) => sum + contrib.amount, 0);
                  const percentage = Math.min((current / g.target_amount) * 100, 100);
                  return (
                    <div key={g.id} className="border border-border rounded p-3">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-sm font-medium">{g.name}</span>
                        <span className="text-sm text-muted-foreground">
                          {formatCurrency(current)} / {formatCurrency(g.target_amount)}
                        </span>
                      </div>
                      <div className="w-full bg-muted rounded-full h-2">
                        <div
                          className="bg-accent h-2 rounded-full transition-all"
                          style={{ width: `${percentage}%` }}
                        />
                      </div>
                      <p className="text-xs text-muted-foreground mt-1">{percentage.toFixed(1)}% complete</p>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

