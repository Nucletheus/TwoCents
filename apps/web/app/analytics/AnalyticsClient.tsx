'use client';

import { useState, useEffect, useMemo } from 'react';
import { useSearchParams } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import {
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  LineChart,
  Line,
  AreaChart,
  Area,
  ComposedChart,
  RadarChart,
  Radar,
  PolarGrid,
  PolarAngleAxis,
  PolarRadiusAxis,
} from 'recharts';
import { formatCurrency, getMonthRange, getYearRange, normalizeCategoryGroupColors } from '@twocents/shared';
import MainLayout from '../components/MainLayout';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Select from '../components/ui/Select';
import Button from '../components/ui/Button';
import Input from '../components/ui/Input';
import { X } from 'lucide-react';
import { useHousehold } from '../components/HouseholdProvider';
import AIInsightsCard from './components/AIInsightsCard';
import SavingsGoalChart from './components/SavingsGoalChart';

export default function AnalyticsClient() {
  const searchParams = useSearchParams();
  const categoryFilter = searchParams.get('category');
  const supabase = createClient();
  const { selectedHouseholdId, loading: householdsLoading } = useHousehold();
  const [expenses, setExpenses] = useState<any[]>([]);
  const [budgets, setBudgets] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [timeframe, setTimeframe] = useState<'month' | 'year' | 'all' | 'custom'>('month');
  const [selectedMonthKeys, setSelectedMonthKeys] = useState<string[]>(() => {
    const now = new Date();
    return [`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`];
  });
  const [customDateFrom, setCustomDateFrom] = useState('');
  const [customDateTo, setCustomDateTo] = useState('');
  const [categories, setCategories] = useState<any[]>([]);
  const [comparisonExpenses, setComparisonExpenses] = useState<any[]>([]);
  const [householdIncome, setHouseholdIncome] = useState<number | null>(null);
  const [incomeLoading, setIncomeLoading] = useState(false);

  const monthOptions = useMemo(() => {
    const now = new Date();
    const items: Array<{ key: string; date: Date; label: string; year: number }> = [];
    for (let i = 0; i < 12; i++) {
      const date = new Date(now.getFullYear(), now.getMonth() - i, 1);
      const key = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`;
      items.push({
        key,
        date,
        label: date.toLocaleDateString('en-US', { month: 'short' }),
        year: date.getFullYear(),
      });
    }
    return items;
  }, []);

  const coerceMonthKeyToDate = (monthKey: string): Date => {
    const match = monthOptions.find((m) => m.key === monthKey);
    if (match) return match.date;
    const [year, month] = monthKey.split('-').map((part) => Number(part));
    if (Number.isFinite(year) && Number.isFinite(month) && month >= 1 && month <= 12) {
      return new Date(year, month - 1, 1);
    }
    return new Date();
  };

  const handleToggleMonth = (monthKey: string) => {
    setTimeframe('month');
    setSelectedMonthKeys((prev) => {
      const current = prev.length > 0 ? prev : [monthKey];
      const set = new Set(current);
      if (set.has(monthKey)) {
        if (set.size === 1) return current;
        set.delete(monthKey);
      } else {
        set.add(monthKey);
      }
      return Array.from(set).sort((a, b) => b.localeCompare(a));
    });
  };

  const handleSelectYear = () => setTimeframe('year');
  const handleSelectAll = () => setTimeframe('all');

  const handleSelectCustom = () => {
    if (!customDateFrom || !customDateTo) {
      if (timeframe === 'month') {
        const dates = (selectedMonthKeys.length > 0 ? selectedMonthKeys : [monthOptions[0]?.key || ''])
          .filter(Boolean)
          .map(coerceMonthKeyToDate)
          .sort((a, b) => a.getTime() - b.getTime());
        const earliest = dates[0] || new Date();
        const latest = dates[dates.length - 1] || new Date();
        const startRange = getMonthRange(earliest);
        const endRange = getMonthRange(latest);
        setCustomDateFrom(startRange.start.toISOString().slice(0, 10));
        setCustomDateTo(endRange.end.toISOString().slice(0, 10));
      } else {
        const range = timeframe === 'year' ? getYearRange() : getMonthRange();
        setCustomDateFrom(range.start.toISOString().slice(0, 10));
        setCustomDateTo(range.end.toISOString().slice(0, 10));
      }
    }
    setTimeframe('custom');
  };

  const resolveDateRange = (): { from?: string; to?: string } => {
    if (timeframe === 'month') {
      const keys = selectedMonthKeys.length > 0 ? selectedMonthKeys : [];
      if (keys.length === 0) return { from: undefined, to: undefined };
      const dates = keys.map(coerceMonthKeyToDate).sort((a, b) => a.getTime() - b.getTime());
      const earliest = dates[0];
      const latest = dates[dates.length - 1];
      const startRange = getMonthRange(earliest);
      const endRange = getMonthRange(latest);
      return {
        from: startRange.start.toISOString().slice(0, 10),
        to: endRange.end.toISOString().slice(0, 10),
      };
    }
    if (timeframe === 'year') {
      const range = getYearRange();
      return {
        from: range.start.toISOString().slice(0, 10),
        to: range.end.toISOString().slice(0, 10),
      };
    }
    if (timeframe === 'custom') {
      return { from: customDateFrom || undefined, to: customDateTo || undefined };
    }
    return { from: undefined, to: undefined };
  };

  useEffect(() => {
    if (selectedHouseholdId) {
      fetchCategories();
      fetchAnalytics();
      fetchHouseholdIncome();
    }
  }, [selectedHouseholdId, timeframe, selectedMonthKeys.join(','), customDateFrom, customDateTo, categoryFilter]);

  const fetchHouseholdIncome = async () => {
    if (!selectedHouseholdId) return;

    setIncomeLoading(true);
    try {
      const currentYear = new Date().getFullYear();
      const { data, error } = await supabase
        .from('household_income')
        .select('annual_income')
        .eq('household_id', selectedHouseholdId)
        .eq('year', currentYear);

      if (error) throw error;

      const totalIncome = (data || []).reduce((sum, row) => sum + Number(row.annual_income || 0), 0);
      setHouseholdIncome(totalIncome > 0 ? totalIncome : null);
    } catch (error) {
      console.error('Error fetching household income:', error);
      setHouseholdIncome(null);
    } finally {
      setIncomeLoading(false);
    }
  };

  useEffect(() => {
    if (!selectedHouseholdId) return;
    const handler = (e: Event) => {
      const detail = (e as CustomEvent)?.detail as { householdId?: string } | undefined;
      if (detail?.householdId && detail.householdId === selectedHouseholdId) {
        fetchCategories();
      }
    };
    window.addEventListener('category-group-colors-updated', handler as EventListener);
    return () => window.removeEventListener('category-group-colors-updated', handler as EventListener);
  }, [selectedHouseholdId]);

  const fetchCategories = async () => {
    if (!selectedHouseholdId) return;
    try {
      const { data, error } = await supabase
        .from('categories')
        .select('*')
        .eq('household_id', selectedHouseholdId)
        .neq('is_hidden', true)
        .order('name');
      if (error) throw error;

      let overrides: Record<string, string> | undefined;
      const { data: overrideRows, error: overrideError } = await supabase
        .from('household_category_group_colors')
        .select('group_name, color')
        .eq('household_id', selectedHouseholdId);

      if (!overrideError && overrideRows) {
        overrides = Object.fromEntries(
          overrideRows.map((row: any) => [String(row.group_name), String(row.color)])
        );
      }

      setCategories(normalizeCategoryGroupColors((data || []) as any, overrides) as any);
    } catch (error) {
      console.error('Error fetching categories:', error);
    }
  };

  const fetchAnalytics = async () => {
    if (!selectedHouseholdId) return;

    setLoading(true);
    try {
      const dateRange = resolveDateRange();
      const periodType = timeframe === 'year' ? 'yearly' : 'monthly';

      let query = supabase
        .from('expenses')
        .select(
          `
          *,
          categories (
            id,
            name,
            icon,
            color
          )
        `
        )
        .eq('household_id', selectedHouseholdId);

      if (dateRange.from) {
        query = query.gte('date', dateRange.from);
      }
      if (dateRange.to) {
        query = query.lte('date', dateRange.to);
      }

      if (categoryFilter) {
        query = query.eq('category_id', categoryFilter);
      }

      const { data: expensesData, error: expensesError } = await query;

      if (expensesError) throw expensesError;

      // Only use the household's main budget (exclude scenarios)
      const { data: mainBudgetSet, error: mainBudgetSetError } = await supabase
        .from('budget_sets')
        .select('id')
        .eq('household_id', selectedHouseholdId)
        .eq('is_main', true)
        .maybeSingle();

      if (!mainBudgetSetError && mainBudgetSet?.id) {
        const { data: budgetsData, error: budgetsError } = await supabase
          .from('budgets')
          .select(
            `
            *,
            categories (
              id,
              name
            )
          `
          )
          .eq('budget_set_id', mainBudgetSet.id)
          .eq('period', periodType);

        if (!budgetsError) {
          setBudgets(budgetsData || []);
        }
      } else {
        setBudgets([]);
      }

      setExpenses(expensesData || []);

      // Fetch comparison period expenses for trend analysis
      if (dateRange.from && dateRange.to) {
        const start = new Date(dateRange.from);
        const end = new Date(dateRange.to);
        const duration = end.getTime() - start.getTime();
        const comparisonStart = new Date(start.getTime() - duration);
        const comparisonEnd = new Date(start.getTime() - 1);

        const comparisonQuery = supabase
          .from('expenses')
          .select(
            `
            *,
            categories (
              id,
              name,
              icon,
              color
            )
          `
          )
          .eq('household_id', selectedHouseholdId)
          .gte('date', comparisonStart.toISOString().split('T')[0])
          .lte('date', comparisonEnd.toISOString().split('T')[0]);

        if (categoryFilter) {
          comparisonQuery.eq('category_id', categoryFilter);
        }

        const { data: comparisonData } = await comparisonQuery;
        setComparisonExpenses(comparisonData || []);
      } else {
        setComparisonExpenses([]);
      }
    } catch (error) {
      console.error('Error fetching analytics:', error);
    } finally {
      setLoading(false);
    }
  };

  const categoryBreakdown = expenses.reduce((acc: any, expense: any) => {
    const categoryName = expense.categories?.name || 'Uncategorized';
    if (!acc[categoryName]) {
      acc[categoryName] = { name: categoryName, amount: 0, color: expense.categories?.color || '#95A5A6' };
    }
    // Use absolute value for display (expenses are stored as negative)
    acc[categoryName].amount += Math.abs(expense.amount);
    return acc;
  }, {});

  const categoryData = Object.values(categoryBreakdown).map((cat: any) => ({
    name: cat.name,
    value: cat.amount,
    color: cat.color,
  }));

  // Prepare radar chart data - show spending as percentage of income
  const radarChartData = useMemo(() => {
    if (!householdIncome || householdIncome <= 0) return [];

    // Calculate monthly income (annual / 12)
    const monthlyIncome = householdIncome / 12;

    // For the selected timeframe, calculate the number of months
    let monthsInTimeframe = 1;
    if (timeframe === 'month') {
      monthsInTimeframe = selectedMonthKeys.length || 1;
    } else if (timeframe === 'year') {
      monthsInTimeframe = 12;
    } else if (timeframe === 'custom' && customDateFrom && customDateTo) {
      const start = new Date(customDateFrom);
      const end = new Date(customDateTo);
      const diffTime = Math.abs(end.getTime() - start.getTime());
      const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));
      monthsInTimeframe = diffDays / 30; // Approximate
    }

    const timeframeIncome = monthlyIncome * monthsInTimeframe;

    // Convert category spending to percentage of income for radar chart
    return categoryData
      .map((cat) => ({
        category: cat.name,
        spending: cat.value,
        percentage: timeframeIncome > 0 ? (cat.value / timeframeIncome) * 100 : 0,
        fullMark: 100, // Maximum percentage (100% of income)
      }))
      .sort((a, b) => b.percentage - a.percentage)
      .slice(0, 8); // Limit to top 8 categories for readability
  }, [categoryData, householdIncome, timeframe, selectedMonthKeys.length, customDateFrom, customDateTo]);

  const monthlySpending = expenses.reduce((acc: any, expense: any) => {
    const month = new Date(expense.date).toLocaleDateString('en-US', {
      month: 'short',
      year: 'numeric',
    });
    if (!acc[month]) {
      acc[month] = 0;
    }
    // Use absolute value for display (expenses are stored as negative)
    acc[month] += Math.abs(expense.amount);
    return acc;
  }, {});

  // Sort monthly data chronologically by parsing the date strings
  const monthlyData = Object.entries(monthlySpending)
    .map(([name, value]) => {
      // Parse "Jan 2024" format to get a sortable date
      const date = new Date(name + ' 1');
      return { name, amount: value, date };
    })
    .sort((a, b) => a.date.getTime() - b.date.getTime())
    .map(({ name, amount }) => ({ name, amount })); // Remove date from final data

  // Calculate comparison period spending
  const comparisonMonthlySpending = comparisonExpenses.reduce((acc: any, expense: any) => {
    const month = new Date(expense.date).toLocaleDateString('en-US', {
      month: 'short',
      year: 'numeric',
    });
    if (!acc[month]) {
      acc[month] = 0;
    }
    // Use absolute value for display (expenses are stored as negative)
    acc[month] += Math.abs(expense.amount);
    return acc;
  }, {});

  // Enhanced monthly data with comparison - sorted chronologically
  const enhancedMonthlyData = useMemo(() => {
    const allMonths = new Set([...Object.keys(monthlySpending), ...Object.keys(comparisonMonthlySpending)]);
    return Array.from(allMonths)
      .map((month) => {
        // Parse "Jan 2024" format to get a sortable date
        const date = new Date(month + ' 1');
        return {
          name: month,
          amount: monthlySpending[month] || 0,
          comparison: comparisonMonthlySpending[month] || 0,
          change: monthlySpending[month] && comparisonMonthlySpending[month]
            ? ((monthlySpending[month] - comparisonMonthlySpending[month]) / comparisonMonthlySpending[month]) * 100
            : null,
          date,
        };
      })
      .sort((a, b) => a.date.getTime() - b.date.getTime())
      .map(({ name, amount, comparison, change }) => ({ name, amount, comparison, change })); // Remove date from final data
  }, [monthlySpending, comparisonMonthlySpending]);

  const budgetComparison = budgets.map((budget) => {
    // Use absolute value for display (expenses are stored as negative)
    const categoryExpenses = Math.abs(expenses
      .filter((e) => e.category_id === budget.category_id)
      .reduce((sum, e) => sum + e.amount, 0));
    return {
      category: budget.categories?.name || 'Unknown',
      budget: budget.amount,
      spent: categoryExpenses,
      remaining: budget.amount - categoryExpenses,
    };
  });

  // Use absolute value for display (expenses are stored as negative)
  const totalSpent = Math.abs(expenses.reduce((sum, e) => sum + e.amount, 0));
  const totalBudget = budgets.reduce((sum, b) => sum + b.amount, 0);

  // Calculate period-over-period change
  // Use absolute value for display (expenses are stored as negative)
  const totalComparisonSpent = Math.abs(comparisonExpenses.reduce((sum, e) => sum + e.amount, 0));
  const periodOverPeriod = totalSpent - totalComparisonSpent;
  const periodOverPeriodPercent =
    totalComparisonSpent > 0 ? ((periodOverPeriod / totalComparisonSpent) * 100).toFixed(1) : '0';

  const COLORS = ['#6366f1', '#8b5cf6', '#ec4899', '#f59e0b', '#10b981', '#3b82f6', '#ef4444'];

  // Calculate spending forecast
  const calculateSpendingForecast = (): Array<{ name: string; actual: number | null; forecast: number | null }> => {
    if (monthlyData.length < 2) return [];

    // Get last 6 months of data for trend calculation
    const recentMonths = monthlyData.slice(-6);
    const monthlyTotals = recentMonths.map((m) => m.amount);

    // Simple linear regression for trend
    const n = monthlyTotals.length;
    const sumX = (n * (n + 1)) / 2; // 1 + 2 + ... + n
    const sumY = monthlyTotals.reduce((sum, val) => sum + val, 0);
    const sumXY = monthlyTotals.reduce((sum, val, idx) => sum + (idx + 1) * val, 0);
    const sumX2 = (n * (n + 1) * (2 * n + 1)) / 6; // 1^2 + 2^2 + ... + n^2

    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);
    const intercept = (sumY - slope * sumX) / n;

    // Create combined data with separate keys for actual and forecast
    const combinedData: Array<{
      name: string;
      actual: number | null;
      forecast: number | null;
    }> = [];

    // Add actual data
    monthlyData.forEach((m) => {
      combinedData.push({
        name: m.name,
        actual: m.amount,
        forecast: null,
      });
    });

    // Add forecast data - start from last actual month to connect smoothly
    const lastMonth = monthlyData[monthlyData.length - 1];
    const lastDate = new Date(lastMonth.name + ' 1');

    // Add a bridge point: last actual month with both actual and forecast values for smooth connection
    if (combinedData.length > 0) {
      const lastActualIndex = combinedData.length - 1;
      const forecastStartValue = intercept + slope * (n + 1);
      combinedData[lastActualIndex] = {
        ...combinedData[lastActualIndex],
        forecast: Math.max(0, forecastStartValue), // Connect forecast from last actual point
      };
    }

    // Project next 6 months
    for (let i = 1; i <= 6; i++) {
      const forecastDate = new Date(lastDate);
      forecastDate.setMonth(forecastDate.getMonth() + i);
      const monthLabel = forecastDate.toLocaleDateString('en-US', { month: 'short', year: 'numeric' });
      const forecastValue = intercept + slope * (n + i);
      combinedData.push({
        name: monthLabel,
        actual: null,
        forecast: Math.max(0, forecastValue), // Ensure non-negative
      });
    }

    return combinedData;
  };

  const spendingForecastData = calculateSpendingForecast();

  // Calculate savings forecast
  const calculateSavingsForecast = () => {
    // Get savings category expenses (positive amounts in Savings & Investments)
    const savingsExpenses = expenses.filter((e) => {
      const groupName = e?.categories?.group_name;
      return groupName === 'Savings & Investments' && Number(e.amount || 0) > 0;
    });

    // Get budgeted savings
    const savingsBudgets = budgets.filter((b) => {
      const groupName = b?.categories?.group_name;
      return groupName === 'Savings & Investments';
    });
    const budgetedSavings = savingsBudgets.reduce((sum, b) => sum + Number(b.amount || 0), 0);

    // Group savings by month
    const monthlySavings: Record<string, { actual: number; budgeted: number }> = {};
    savingsExpenses.forEach((expense) => {
      const month = new Date(expense.date).toLocaleDateString('en-US', {
        month: 'short',
        year: 'numeric',
      });
      if (!monthlySavings[month]) {
        monthlySavings[month] = { actual: 0, budgeted: 0 };
      }
      monthlySavings[month].actual += Number(expense.amount || 0);
    });

    // Add budgeted amounts (assuming monthly period)
    Object.keys(monthlySavings).forEach((month) => {
      monthlySavings[month].budgeted = budgetedSavings;
    });

    // Convert to array and sort
    const savingsData = Object.entries(monthlySavings)
      .map(([name, values]) => ({
        name,
        actual: values.actual,
        budgeted: values.budgeted,
      }))
      .sort((a, b) => a.name.localeCompare(b.name));

    // Calculate average savings rate
    const avgActual = savingsData.length > 0
      ? savingsData.reduce((sum, d) => sum + d.actual, 0) / savingsData.length
      : 0;
    const avgBudgeted = budgetedSavings;

    // Project future months
    const forecastData = [...savingsData];
    if (savingsData.length > 0) {
      const lastMonth = savingsData[savingsData.length - 1];
      const lastDate = new Date(lastMonth.name);

      // Project next 6 months
      for (let i = 1; i <= 6; i++) {
        const forecastDate = new Date(lastDate);
        forecastDate.setMonth(forecastDate.getMonth() + i);
        const monthLabel = forecastDate.toLocaleDateString('en-US', { month: 'short', year: 'numeric' });
        forecastData.push({
          name: monthLabel,
          actual: avgActual,
          budgeted: avgBudgeted,
          isForecast: true,
        });
      }
    }

    return forecastData;
  };

  const savingsForecastData = calculateSavingsForecast();

  const selectedCategory = categoryFilter ? categories.find((c) => c.id === categoryFilter) : null;
  const subtitle = categoryFilter
    ? `Spending for ${selectedCategory?.name || 'category'}`
    : 'Detailed spending insights';
  const isLoading = householdsLoading || (selectedHouseholdId ? loading : false);

  // Calculate timeframe key for insights
  const getTimeframeKey = (): string => {
    if (timeframe === 'month') {
      return selectedMonthKeys.length > 0 ? selectedMonthKeys.sort().join(',') : '';
    }
    if (timeframe === 'year') {
      return new Date().getFullYear().toString();
    }
    if (timeframe === 'custom') {
      return `${customDateFrom || ''}_${customDateTo || ''}`;
    }
    return 'all';
  };

  return (
    <MainLayout
      pageHeader={{
        title: 'Analytics',
        subtitle,
        secondary: (
          <div className="w-full space-y-2">
            <div className="flex items-center gap-2">
              {categoryFilter && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    window.history.pushState({}, '', '/analytics');
                    window.location.reload();
                  }}
                >
                  <X className="mr-2 h-4 w-4" />
                  Clear Category Filter
                </Button>
              )}
            </div>
            <div className="overflow-x-auto scrollbar-hide">
              <div className="flex w-max items-center gap-2 mx-auto py-0.5">
                {monthOptions.map((option) => {
                  const isSelected = timeframe === 'month' && selectedMonthKeys.includes(option.key);
                  const nowYear = new Date().getFullYear();
                  const showYear = option.year !== nowYear;
                  return (
                    <Button
                      key={option.key}
                      type="button"
                      size="sm"
                      variant={isSelected ? 'primary' : 'secondary'}
                      onClick={() => handleToggleMonth(option.key)}
                      title={`${option.label} ${option.year}`}
                      className="h-auto px-3 py-1 shrink-0 focus:ring-1 focus:ring-offset-0"
                    >
                      <span className="flex flex-col leading-tight">
                        <span>{option.label}</span>
                        {showYear && <span className="text-[10px] text-muted-foreground">{option.year}</span>}
                      </span>
                    </Button>
                  );
                })}

                <div className="mx-1 h-5 w-px bg-border/30 self-center" />

                <Button
                  type="button"
                  size="sm"
                  variant={timeframe === 'year' ? 'primary' : 'secondary'}
                  onClick={handleSelectYear}
                  className="shrink-0 focus:ring-1 focus:ring-offset-0"
                >
                  This Year
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant={timeframe === 'all' ? 'primary' : 'secondary'}
                  onClick={handleSelectAll}
                  className="shrink-0 focus:ring-1 focus:ring-offset-0"
                >
                  All Time
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant={timeframe === 'custom' ? 'primary' : 'secondary'}
                  onClick={handleSelectCustom}
                  className="shrink-0 focus:ring-1 focus:ring-offset-0"
                >
                  Custom
                </Button>
              </div>
            </div>

            {timeframe === 'custom' && (
              <div className="grid gap-3 sm:grid-cols-2">
                <div>
                  <label className="mb-1 block text-sm font-medium">From</label>
                  <Input
                    type="date"
                    value={customDateFrom}
                    onChange={(e) => setCustomDateFrom(e.target.value)}
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm font-medium">To</label>
                  <Input type="date" value={customDateTo} onChange={(e) => setCustomDateTo(e.target.value)} />
                </div>
              </div>
            )}
          </div>
        ),
      }}
    >
      {isLoading ? (
        <div className="flex items-center justify-center py-24">
          <div className="text-muted-foreground">Loading analytics...</div>
        </div>
      ) : !selectedHouseholdId ? (
        <Card>
          <CardContent className="py-8 text-center">
            <p className="text-muted-foreground">Please select or create a household first.</p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-6">
            {/* Summary Cards */}
            <div className="grid gap-4 md:grid-cols-3">
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm font-medium">Total Spent</CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-3xl font-bold">{formatCurrency(totalSpent)}</p>
                  {totalComparisonSpent > 0 && (
                    <p className={`text-xs mt-1 ${periodOverPeriod >= 0 ? 'text-red-600' : 'text-green-600'}`}>
                      {periodOverPeriod >= 0 ? '+' : ''}
                      {periodOverPeriodPercent}% vs previous period
                    </p>
                  )}
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm font-medium">Total Budget</CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-3xl font-bold">{formatCurrency(totalBudget)}</p>
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm font-medium">Remaining</CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-3xl font-bold">{formatCurrency(totalBudget - totalSpent)}</p>
                </CardContent>
              </Card>
            </div>

            {/* AI Insights */}
            {selectedHouseholdId && (
              <AIInsightsCard
                householdId={selectedHouseholdId}
                timeframe={timeframe}
                timeframeKey={getTimeframeKey()}
              />
            )}

            {/* Category Breakdown - Radar Chart */}
            {categoryData.length > 0 && (
              <Card>
                <CardHeader>
                  <CardTitle>Category Breakdown</CardTitle>
                  <CardDescription>
                    Spending by category as percentage of household income
                    {householdIncome && ` (${formatCurrency(householdIncome)}/year)`}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {incomeLoading ? (
                    <div className="flex items-center justify-center h-[300px]">
                      <div className="text-muted-foreground">Loading income data...</div>
                    </div>
                  ) : !householdIncome || householdIncome <= 0 ? (
                    <div className="flex flex-col items-center justify-center h-[300px] text-center space-y-4">
                      <div className="text-muted-foreground">
                        <p className="font-medium mb-2">Household income not set</p>
                        <p className="text-sm">
                          To view spending as a percentage of income, please set your household income in the Budgets
                          page.
                        </p>
                      </div>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => {
                          window.location.href = '/budgets';
                        }}
                      >
                        Go to Budgets
                      </Button>
                    </div>
                  ) : radarChartData.length > 0 ? (
                    <ResponsiveContainer width="100%" height={400}>
                      <RadarChart data={radarChartData}>
                        <PolarGrid />
                        <PolarAngleAxis dataKey="category" className="text-xs" />
                        <PolarRadiusAxis
                          angle={90}
                          domain={[0, 100]}
                          tickFormatter={(value) => `${value}%`}
                          className="text-xs"
                        />
                        <Radar
                          name="Spending % of Income"
                          dataKey="percentage"
                          stroke="#6366f1"
                          fill="#6366f1"
                          fillOpacity={0.6}
                        />
                        <Tooltip
                          formatter={(value: number, name: string, props: any) => [
                            `${value.toFixed(1)}% (${formatCurrency(props.payload.spending)})`,
                            'Spending % of Income',
                          ]}
                          labelFormatter={(label) => label}
                        />
                        <Legend />
                      </RadarChart>
                    </ResponsiveContainer>
                  ) : (
                    <div className="flex items-center justify-center h-[300px]">
                      <div className="text-muted-foreground">No category data available</div>
                    </div>
                  )}
                </CardContent>
              </Card>
            )}

            {/* Monthly Spending with Comparison */}
            {enhancedMonthlyData.length > 0 && (
              <Card>
                <CardHeader>
                  <CardTitle>Spending Over Time</CardTitle>
                  <CardDescription>Monthly trends with period comparison</CardDescription>
                </CardHeader>
                <CardContent>
                  <ResponsiveContainer width="100%" height={300}>
                    <AreaChart data={enhancedMonthlyData}>
                      <CartesianGrid strokeDasharray="3 3" />
                      <XAxis dataKey="name" />
                      <YAxis />
                      <Tooltip
                        formatter={(value: number, name: string, props: any) => {
                          if (name === 'change' && props.payload.change !== null) {
                            return `${props.payload.change >= 0 ? '+' : ''}${props.payload.change.toFixed(1)}%`;
                          }
                          return formatCurrency(value);
                        }}
                        labelFormatter={(label) => label}
                        contentStyle={{
                          backgroundColor: 'var(--card)',
                          border: '1px solid var(--border)',
                          borderRadius: '3px',
                        }}
                      />
                      <Legend />
                      {totalComparisonSpent > 0 && (
                        <Area
                          type="monotone"
                          dataKey="comparison"
                          stroke="#94a3b8"
                          fill="#94a3b8"
                          fillOpacity={0.3}
                          name="Previous Period"
                        />
                      )}
                      <Area
                        type="monotone"
                        dataKey="amount"
                        stroke="#6366f1"
                        fill="#6366f1"
                        fillOpacity={0.6}
                        name="Current Period"
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                  {totalComparisonSpent > 0 && (
                    <p className="text-xs text-muted-foreground mt-2">
                      Comparison shows spending from the equivalent previous period
                    </p>
                  )}
                </CardContent>
              </Card>
            )}

            {/* Spending Forecast */}
            {spendingForecastData.length > 0 && (
              <Card>
                <CardHeader>
                  <CardTitle>Spending Forecast</CardTitle>
                  <CardDescription>Current spending and projected future spending based on trends</CardDescription>
                </CardHeader>
                <CardContent>
                  <ResponsiveContainer width="100%" height={300}>
                    <ComposedChart data={spendingForecastData}>
                      <CartesianGrid strokeDasharray="3 3" />
                      <XAxis dataKey="name" />
                      <YAxis />
                      <Tooltip
                        formatter={(value: number) => (value ? formatCurrency(value) : '')}
                        labelFormatter={(label) => label}
                        contentStyle={{
                          backgroundColor: 'var(--card)',
                          border: '1px solid var(--border)',
                          borderRadius: '3px',
                        }}
                      />
                      <Legend />
                      {/* Actual spending as filled area */}
                      <Area
                        type="monotone"
                        dataKey="actual"
                        stroke="#6366f1"
                        fill="#6366f1"
                        fillOpacity={0.6}
                        name="Actual Spending"
                        connectNulls={false}
                      />
                      {/* Forecast as dashed line */}
                      <Line
                        type="monotone"
                        dataKey="forecast"
                        stroke="#6366f1"
                        strokeWidth={2}
                        strokeDasharray="5 5"
                        name="Forecast"
                        dot={{ fill: '#6366f1', r: 3 }}
                        connectNulls={true}
                      />
                    </ComposedChart>
                  </ResponsiveContainer>
                  <p className="text-xs text-muted-foreground mt-2">
                    Filled area shows actual spending for selected timeframe. Dashed line indicates forecasted values.
                  </p>
                </CardContent>
              </Card>
            )}

            {/* Savings Forecast */}
            {savingsForecastData.length > 0 && (
              <Card>
                <CardHeader>
                  <CardTitle>Savings Forecast</CardTitle>
                  <CardDescription>Budgeted vs actual savings over time</CardDescription>
                </CardHeader>
                <CardContent>
                  <ResponsiveContainer width="100%" height={300}>
                    <AreaChart data={savingsForecastData}>
                      <CartesianGrid strokeDasharray="3 3" />
                      <XAxis dataKey="name" />
                      <YAxis />
                      <Tooltip formatter={(value: number) => formatCurrency(value)} />
                      <Legend />
                      <Area
                        type="monotone"
                        dataKey="budgeted"
                        stackId="1"
                        stroke="#10b981"
                        fill="#10b981"
                        fillOpacity={0.3}
                        name="Budgeted Savings"
                        strokeDasharray={savingsForecastData[savingsForecastData.length - 1]?.isForecast ? '5 5' : '0'}
                      />
                      <Area
                        type="monotone"
                        dataKey="actual"
                        stackId="2"
                        stroke="#3b82f6"
                        fill="#3b82f6"
                        fillOpacity={0.3}
                        name="Actual Savings"
                        strokeDasharray={savingsForecastData[savingsForecastData.length - 1]?.isForecast ? '5 5' : '0'}
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                  <p className="text-xs text-muted-foreground mt-2">
                    Dashed lines indicate forecasted values
                  </p>
                </CardContent>
              </Card>
            )}

            {/* Savings Goals Progress */}
            {selectedHouseholdId && (
              <SavingsGoalChart
                householdId={selectedHouseholdId}
                expenses={expenses}
                budgets={budgets}
                categories={categories}
              />
            )}

            {/* Budget Comparison */}
            {budgetComparison.length > 0 && (
              <Card>
                <CardHeader>
                  <CardTitle>Budget vs Actual</CardTitle>
                  <CardDescription>Category-wise comparison</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="overflow-x-auto">
                    <table className="w-full border-collapse">
                      <thead>
                        <tr className="border-b border-border">
                          <th className="p-3 text-left text-sm font-medium">Category</th>
                          <th className="p-3 text-left text-sm font-medium">Budget</th>
                          <th className="p-3 text-left text-sm font-medium">Spent</th>
                          <th className="p-3 text-left text-sm font-medium">Remaining</th>
                        </tr>
                      </thead>
                      <tbody>
                        {budgetComparison.map((item, index) => (
                          <tr key={index} className="border-b border-border hover:bg-hover">
                            <td className="p-3 text-sm font-medium">{item.category}</td>
                            <td className="p-3 text-sm text-muted-foreground">{formatCurrency(item.budget)}</td>
                            <td className="p-3 text-sm text-muted-foreground">{formatCurrency(item.spent)}</td>
                            <td
                              className={`p-3 text-sm font-medium ${
                                item.remaining >= 0 ? 'text-green-600' : 'text-red-600'
                              }`}
                            >
                              {formatCurrency(item.remaining)}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </CardContent>
              </Card>
            )}
        </div>
      )}
    </MainLayout>
  );
}

