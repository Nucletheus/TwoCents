'use client';

import { useState, useEffect, useMemo } from 'react';
import { createClient } from '@/lib/supabase/client';
import { formatCurrency, getMonthRange, getYearRange } from '@twocents/shared';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Button from '../components/ui/Button';
import Input from '../components/ui/Input';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog';
import { TrendingUp, TrendingDown, DollarSign, Target, Receipt, Flag, AlertCircle } from 'lucide-react';
import MainLayout from '../components/MainLayout';
import { useHousehold } from '../components/HouseholdProvider';
import ReviewGrid from '../expenses/ReviewGrid';
import type { Category } from '@twocents/shared';

interface HouseholdMember {
  user_id: string;
  profiles: {
    name: string | null;
    email: string;
  } | null;
}

interface Account {
  id: string;
  name: string;
  type: string;
}

export default function DashboardClient() {
  const supabase = createClient();
  const { selectedHouseholdId } = useHousehold();
  const [expenses, setExpenses] = useState<any[]>([]);
  const [budgetedSpending, setBudgetedSpending] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [timeframe, setTimeframe] = useState<'month' | 'year' | 'all' | 'custom'>('month');
  const [selectedMonthKeys, setSelectedMonthKeys] = useState<string[]>(() => {
    const now = new Date();
    return [`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`];
  });
  const [customDateFrom, setCustomDateFrom] = useState('');
  const [customDateTo, setCustomDateTo] = useState('');
  const [flagDialogOpen, setFlagDialogOpen] = useState(false);
  const [selectedExpenseForFlag, setSelectedExpenseForFlag] = useState<any | null>(null);
  const [flagVendorFuture, setFlagVendorFuture] = useState(false);
  const [householdMembers, setHouseholdMembers] = useState<HouseholdMember[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [comparisonExpenses, setComparisonExpenses] = useState<any[]>([]);

  const isExcludedFromSpending = (
    groupName: string | null | undefined, 
    category: { name?: string | null; exclude_from_calculations?: boolean; is_income_category?: boolean } | null | undefined
  ) => {
    // Check the exclude_from_calculations field first
    if (category?.exclude_from_calculations) return true;
    
    // Income categories are excluded from spending calculations
    if (category?.is_income_category) return true;
    
    if (!groupName) return false;
    if (groupName === 'Savings & Investments') return true;
    if (groupName === 'Transfers') return true;
    if (groupName.startsWith('Income')) return true;
    return false;
  };

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
      fetchExpenses();
      fetchComparisonExpenses();
      fetchHouseholdMembers();
      fetchAccounts();
      fetchCategories();
    }
  }, [selectedHouseholdId, timeframe, selectedMonthKeys.join(','), customDateFrom, customDateTo]);

  const fetchHouseholdMembers = async () => {
    if (!selectedHouseholdId) {
      setHouseholdMembers([]);
      return;
    }

    try {
      const { data, error } = await supabase
        .from('household_members')
        .select(`user_id, profiles!inner (id, name)`)
        .eq('household_id', selectedHouseholdId);

      if (error) {
        console.warn('Error with relationship query, trying alternative:', error);
        const { data: membersData, error: membersError } = await supabase
          .from('household_members')
          .select('user_id')
          .eq('household_id', selectedHouseholdId);

        if (membersError) {
          console.error('Error fetching household members:', membersError);
          setHouseholdMembers([]);
          return;
        }

        if (!membersData || membersData.length === 0) {
          setHouseholdMembers([]);
          return;
        }

        const userIds = membersData.map((m: any) => m.user_id);
        const { data: profilesData, error: profilesError } = await supabase
          .from('profiles')
          .select('id, name')
          .in('id', userIds);

        if (profilesError) {
          console.error('Error fetching profiles:', profilesError);
          setHouseholdMembers([]);
          return;
        }

        const { data: { user: currentUser } } = await supabase.auth.getUser();
        const profilesMap = new Map((profilesData || []).map((p: any) => [p.id, p]));

        const members = membersData.map((member: any) => {
          const profile = profilesMap.get(member.user_id);
          return {
            user_id: member.user_id,
            profiles: profile ? {
              name: profile.name,
              email: currentUser?.id === member.user_id ? (currentUser.email || '') : 'User'
            } : null,
          };
        });

        setHouseholdMembers(members);
        return;
      }

      const members = (data || []).map((member: any) => {
        const profile = Array.isArray(member.profiles) ? member.profiles[0] : member.profiles;
        return {
          user_id: member.user_id,
          profiles: profile ? {
            name: profile.name,
            email: 'User'
          } : null,
        };
      });

      setHouseholdMembers(members);
    } catch (error) {
      console.error('Error fetching household members:', error);
      setHouseholdMembers([]);
    }
  };

  const fetchAccounts = async () => {
    if (!selectedHouseholdId) {
      setAccounts([]);
      return;
    }
    try {
      const { data, error } = await supabase
        .from('accounts')
        .select('id, name, type')
        .eq('household_id', selectedHouseholdId)
        .order('name');

      if (error) throw error;
      setAccounts(data || []);
    } catch (error) {
      console.error('Error fetching accounts:', error);
    }
  };

  const fetchCategories = async () => {
    if (!selectedHouseholdId) {
      setCategories([]);
      return;
    }
    try {
      const { data, error } = await supabase
        .from('categories')
        .select('*')
        .eq('household_id', selectedHouseholdId)
        .order('group_name, name');

      if (error) throw error;
      setCategories(data || []);
    } catch (error) {
      console.error('Error fetching categories:', error);
    }
  };

  const fetchExpenses = async () => {
    if (!selectedHouseholdId) return;

    setLoading(true);
    try {
      const dateRange = resolveDateRange();
      let query = supabase
        .from('expenses')
        .select(
          `
          *,
          categories (
            id,
            name,
            icon,
            color,
            group_name,
            exclude_from_calculations,
            is_income_category
          )
        `
        )
        .eq('household_id', selectedHouseholdId)
        .order('date', { ascending: false });

      if (dateRange.from) {
        query = query.gte('date', dateRange.from);
      }
      if (dateRange.to) {
        query = query.lte('date', dateRange.to);
      }

      const { data: expensesData, error } = await query;

      if (error) throw error;
      setExpenses(expensesData || []);

      // Fetch main budget totals for this timeframe
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
            amount,
            categories (
              group_name,
              name,
              exclude_from_calculations,
              is_income_category
            )
          `
          )
          .eq('budget_set_id', mainBudgetSet.id)
          .eq('period', 'monthly');

        if (!budgetsError) {
          const totalBudgetedSpending =
            (budgetsData || []).reduce((sum: number, b: any) => {
              const groupName = b?.categories?.group_name ?? null;
              const category = b?.categories ?? null;
              if (isExcludedFromSpending(groupName, category)) return sum;
              return sum + Number(b.amount ?? 0);
            }, 0) || 0;
          setBudgetedSpending(totalBudgetedSpending);
        } else {
          setBudgetedSpending(0);
        }
      } else {
        setBudgetedSpending(0);
      }
    } catch (error) {
      console.error('Error fetching expenses:', error);
    } finally {
      setLoading(false);
    }
  };

  const fetchComparisonExpenses = async () => {
    if (!selectedHouseholdId) return;

    try {
      const dateRange = resolveDateRange();
      if (!dateRange.from || !dateRange.to || timeframe === 'all') {
        setComparisonExpenses([]);
        return;
      }

      const start = new Date(dateRange.from);
      const end = new Date(dateRange.to);
      const duration = end.getTime() - start.getTime();
      const comparisonStart = new Date(start.getTime() - duration);
      const comparisonEnd = new Date(start.getTime() - 1);

      const { data: comparisonData, error } = await supabase
        .from('expenses')
        .select(
          `
          *,
          categories (
            id,
            name,
            icon,
            color,
            group_name,
            exclude_from_calculations,
            is_income_category
          )
        `
        )
        .eq('household_id', selectedHouseholdId)
        .gte('date', comparisonStart.toISOString().split('T')[0])
        .lte('date', comparisonEnd.toISOString().split('T')[0])
        .order('date', { ascending: false });

      if (error) throw error;
      setComparisonExpenses(comparisonData || []);
    } catch (error) {
      console.error('Error fetching comparison expenses:', error);
      setComparisonExpenses([]);
    }
  };


  const handleFlagTransaction = (expense: any) => {
    setSelectedExpenseForFlag(expense);
    setFlagVendorFuture(false);
    setFlagDialogOpen(true);
  };

  const confirmFlagTransaction = async () => {
    if (!selectedExpenseForFlag || !selectedHouseholdId) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      // Create expense flag
      const { error: flagError } = await supabase.from('expense_flags').insert({
        expense_id: selectedExpenseForFlag.id,
        user_id: user.id,
        flag_type: 'review',
        notes: 'Flagged for review',
      });

      if (flagError) throw flagError;

      // Update expense status
      const { error: statusError } = await supabase
        .from('expenses')
        .update({ status: 'flagged' })
        .eq('id', selectedExpenseForFlag.id);

      if (statusError) throw statusError;

      // If user wants to flag future transactions from this vendor
      if (flagVendorFuture && selectedExpenseForFlag.vendor) {
        const { error: vendorError } = await supabase
          .from('vendor_review_flags')
          .upsert(
            {
              household_id: selectedHouseholdId,
              vendor_name: selectedExpenseForFlag.vendor,
              requires_review: true,
            },
            { onConflict: 'household_id,vendor_name' }
          );

        if (vendorError) console.error('Error flagging vendor:', vendorError);
      }

      await fetchExpenses();
      setFlagDialogOpen(false);
      setSelectedExpenseForFlag(null);
    } catch (error) {
      console.error('Error flagging transaction:', error);
    }
  };

  // Calculate metrics
  const totalSpent = expenses.reduce((sum, e) => sum + e.amount, 0);
  const actualSpendingForBudget = expenses.reduce((sum, e) => {
    const groupName = e?.categories?.group_name ?? null;
    const category = e?.categories ?? null;
    if (isExcludedFromSpending(groupName, category)) return sum;
    return sum + Number(e.amount ?? 0);
  }, 0);
  const budgetRemaining = (budgetedSpending ?? 0) - actualSpendingForBudget;

  // Get comparison period expenses
  const [comparisonSpent, setComparisonSpent] = useState(0);

  useEffect(() => {
    if (selectedHouseholdId) {
      const dateRange = resolveDateRange();
      if (!dateRange.from || !dateRange.to) {
        setComparisonSpent(0);
        return;
      }

      const start = new Date(dateRange.from);
      const end = new Date(dateRange.to);
      const duration = end.getTime() - start.getTime();
      const comparisonStart = new Date(start.getTime() - duration);
      const comparisonEnd = new Date(start.getTime() - 1);

      supabase
        .from('expenses')
        .select('amount')
        .eq('household_id', selectedHouseholdId)
        .gte('date', comparisonStart.toISOString().split('T')[0])
        .lte('date', comparisonEnd.toISOString().split('T')[0])
        .then(({ data }) => {
          const sum = data?.reduce((acc, e) => acc + e.amount, 0) || 0;
          setComparisonSpent(sum);
        });
    }
  }, [selectedHouseholdId, timeframe, selectedMonthKeys.join(','), customDateFrom, customDateTo]);

  const periodOverPeriod = totalSpent - comparisonSpent;
  const periodOverPeriodPercent =
    comparisonSpent > 0 ? ((periodOverPeriod / comparisonSpent) * 100).toFixed(1) : '0';

  // Calculate top 3 increasing categories
  const calculateTopIncreasingCategories = () => {
    if (timeframe === 'all' || !comparisonExpenses.length) {
      return [];
    }

    // Group current period expenses by category
    const currentByCategory = expenses.reduce((acc: any, expense: any) => {
      const categoryName = expense.categories?.name || 'Uncategorized';
      if (!acc[categoryName]) {
        acc[categoryName] = {
          name: categoryName,
          amount: 0,
          color: expense.categories?.color || '#95A5A6',
        };
      }
      acc[categoryName].amount += expense.amount;
      return acc;
    }, {});

    // Group comparison period expenses by category
    const previousByCategory = comparisonExpenses.reduce((acc: any, expense: any) => {
      const categoryName = expense.categories?.name || 'Uncategorized';
      if (!acc[categoryName]) {
        acc[categoryName] = {
          name: categoryName,
          amount: 0,
        };
      }
      acc[categoryName].amount += expense.amount;
      return acc;
    }, {});

    // Calculate increases and filter to positive increases
    const categoryIncreases = Object.keys(currentByCategory)
      .map((categoryName) => {
        const current = currentByCategory[categoryName];
        const previous = previousByCategory[categoryName] || { amount: 0 };
        const increase = current.amount - previous.amount;

        return {
          name: categoryName,
          color: current.color,
          currentAmount: current.amount,
          previousAmount: previous.amount,
          increase,
        };
      })
      .filter((cat) => cat.increase > 0)
      .sort((a, b) => b.increase - a.increase)
      .slice(0, 3);

    return categoryIncreases;
  };

  const topIncreasingCategories = calculateTopIncreasingCategories();

  // Build time-series data for area chart
  const buildCategoryTrendData = () => {
    if (topIncreasingCategories.length === 0) {
      return [];
    }

    const categoryNames = topIncreasingCategories.map((cat) => cat.name);
    
    // Group expenses by date and category
    const expensesByDate: Record<string, Record<string, number>> = {};
    
    expenses.forEach((expense: any) => {
      const categoryName = expense.categories?.name || 'Uncategorized';
      if (!categoryNames.includes(categoryName)) return;

      const date = expense.date;
      if (!expensesByDate[date]) {
        expensesByDate[date] = {};
        categoryNames.forEach((name) => {
          expensesByDate[date][name] = 0;
        });
      }
      expensesByDate[date][categoryName] = (expensesByDate[date][categoryName] || 0) + expense.amount;
    });

    // Convert to array format with daily spending (not cumulative)
    const dates = Object.keys(expensesByDate).sort();
    const trendData: Array<Record<string, any>> = [];

    dates.forEach((date) => {
      const dataPoint: Record<string, any> = {
        date: new Date(date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' }),
        rawDate: date,
      };

      categoryNames.forEach((name) => {
        dataPoint[name] = expensesByDate[date][name] || 0;
      });

      trendData.push(dataPoint);
    });

    return trendData;
  };

  const categoryTrendData = buildCategoryTrendData();

  // Category breakdown
  const categoryBreakdown = expenses.reduce((acc: any, expense: any) => {
    const categoryName = expense.categories?.name || 'Uncategorized';
    if (!acc[categoryName]) {
      acc[categoryName] = {
        name: categoryName,
        amount: 0,
        color: expense.categories?.color || '#95A5A6',
      };
    }
    acc[categoryName].amount += expense.amount;
    return acc;
  }, {});

  const categoryData = Object.values(categoryBreakdown).map((cat: any) => ({
    name: cat.name,
    value: cat.amount,
    color: cat.color,
  }));

  // Daily spending trend
  const dailySpending = expenses.reduce((acc: any, expense: any) => {
    const date = expense.date;
    if (!acc[date]) {
      acc[date] = 0;
    }
    acc[date] += expense.amount;
    return acc;
  }, {});

  const dailyData = Object.entries(dailySpending)
    .map(([date, amount]) => ({
      date: new Date(date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' }),
      amount,
    }))
    .sort((a, b) => new Date(a.date).getTime() - new Date(b.date).getTime());

  return (
    <MainLayout
      pageHeader={{
        title: 'Dashboard',
        subtitle: 'Overview of your household finances',
        secondary: (
          <div className="w-full space-y-2">
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
      {loading ? (
        <div className="flex items-center justify-center py-24">
          <div className="text-muted-foreground">Loading dashboard...</div>
        </div>
      ) : (
        <div className="space-y-6">
          {!selectedHouseholdId ? (
            <Card>
              <CardContent className="py-8 text-center">
                <p className="text-muted-foreground">Please select or create a household first.</p>
              </CardContent>
            </Card>
          ) : (
            <>
              {/* Key Metrics */}
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">Total Spent</CardTitle>
                    <DollarSign className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">{formatCurrency(totalSpent)}</div>
                    <div className="flex items-center text-xs text-muted-foreground">
                      {periodOverPeriod >= 0 ? (
                        <TrendingUp className="mr-1 h-3 w-3 text-red-600" />
                      ) : (
                        <TrendingDown className="mr-1 h-3 w-3 text-green-600" />
                      )}
                      <span className={periodOverPeriod >= 0 ? 'text-red-600' : 'text-green-600'}>
                        {Math.abs(parseFloat(periodOverPeriodPercent))}% from previous period
                      </span>
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">Transactions</CardTitle>
                    <Receipt className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">{expenses.length}</div>
                    <p className="text-xs text-muted-foreground">expenses recorded</p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">Avg Daily</CardTitle>
                    <TrendingUp className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">
                      {formatCurrency(dailyData.length > 0 ? totalSpent / dailyData.length : 0)}
                    </div>
                    <p className="text-xs text-muted-foreground">per day</p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">Budget Remaining</CardTitle>
                    <Target className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className={`text-2xl font-bold ${budgetRemaining >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                      {formatCurrency(budgetRemaining)}
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {formatCurrency(actualSpendingForBudget)} of {formatCurrency(budgetedSpending ?? 0)} spent
                    </p>
                  </CardContent>
                </Card>
              </div>

              {/* Charts Row */}
              <div className="grid gap-4 md:grid-cols-2">
                {/* Spending Trend */}
                <Card>
                  <CardHeader>
                    <CardTitle>Spending Trend</CardTitle>
                    <CardDescription>Daily spending for selected timeframe</CardDescription>
                  </CardHeader>
                  <CardContent>
                    {dailyData.length > 0 ? (
                      <ResponsiveContainer width="100%" height={300}>
                        <AreaChart data={dailyData}>
                          <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                          <XAxis dataKey="date" className="text-xs" />
                          <YAxis className="text-xs" />
                          <Tooltip
                            formatter={(value: number) => formatCurrency(value)}
                            contentStyle={{
                              backgroundColor: 'var(--card)',
                              border: '1px solid var(--border)',
                              borderRadius: '3px',
                            }}
                          />
                          <Area
                            type="monotone"
                            dataKey="amount"
                            stroke="var(--accent)"
                            fill="var(--accent)"
                            fillOpacity={0.2}
                          />
                        </AreaChart>
                      </ResponsiveContainer>
                    ) : (
                      <div className="flex h-[300px] items-center justify-center text-muted-foreground">
                        No data available
                      </div>
                    )}
                  </CardContent>
                </Card>

                {/* Category Breakdown */}
                <Card>
                  <CardHeader>
                    <CardTitle>Category Breakdown</CardTitle>
                    <CardDescription>Spending by category</CardDescription>
                  </CardHeader>
                  <CardContent>
                    {categoryData.length > 0 ? (
                      <ResponsiveContainer width="100%" height={300}>
                        <PieChart>
                          <Pie
                            data={categoryData}
                            cx="50%"
                            cy="50%"
                            labelLine={false}
                            label={({ name, percent }) => `${name} ${(percent * 100).toFixed(0)}%`}
                            outerRadius={80}
                            fill="#8884d8"
                            dataKey="value"
                          >
                            {categoryData.map((entry, index) => (
                              <Cell key={`cell-${index}`} fill={entry.color || '#95A5A6'} />
                            ))}
                          </Pie>
                          <Tooltip formatter={(value: number) => formatCurrency(value)} />
                        </PieChart>
                      </ResponsiveContainer>
                    ) : (
                      <div className="flex h-[300px] items-center justify-center text-muted-foreground">
                        No data available
                      </div>
                    )}
                  </CardContent>
                </Card>
              </div>

              {/* Flagged Transactions Review */}
              <ReviewGrid
                householdId={selectedHouseholdId}
                categories={categories}
                householdMembers={householdMembers}
                accounts={accounts}
                onExpenseUpdated={() => {
                  fetchExpenses();
                }}
              />

              {/* Spending Trends */}
              <Card>
                <CardHeader>
                  <CardTitle>Spending Trends</CardTitle>
                  <CardDescription>
                    Top {topIncreasingCategories.length} categories with largest spending increases
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {topIncreasingCategories.length > 0 && categoryTrendData.length > 0 ? (
                    <ResponsiveContainer width="100%" height={300}>
                      <AreaChart data={categoryTrendData}>
                        <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                        <XAxis dataKey="date" className="text-xs" />
                        <YAxis className="text-xs" />
                        <Tooltip
                          formatter={(value: number) => formatCurrency(value)}
                          contentStyle={{
                            backgroundColor: 'var(--card)',
                            border: '1px solid var(--border)',
                            borderRadius: '3px',
                          }}
                        />
                        <Legend />
                        {topIncreasingCategories.map((category) => (
                          <Area
                            key={category.name}
                            type="monotone"
                            dataKey={category.name}
                            stroke={category.color}
                            fill={category.color}
                            fillOpacity={0.4}
                            name={category.name}
                          />
                        ))}
                      </AreaChart>
                    </ResponsiveContainer>
                  ) : (
                    <div className="flex h-[300px] items-center justify-center text-muted-foreground">
                      {timeframe === 'all'
                        ? 'Comparison not available for "All Time" timeframe'
                        : topIncreasingCategories.length === 0
                          ? 'No categories showing increases compared to previous period'
                          : 'No data available'}
                    </div>
                  )}
                </CardContent>
              </Card>

            </>
          )}
        </div>
      )}

      {/* Flag Transaction Dialog */}
      <Dialog open={flagDialogOpen} onOpenChange={setFlagDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Flag Transaction for Review</DialogTitle>
            <DialogDescription>
              This will mark the transaction for review by other household members.
            </DialogDescription>
          </DialogHeader>
          {selectedExpenseForFlag && (
            <div className="space-y-4">
              <div className="rounded-md border border-border p-3">
                <div className="font-medium">{formatCurrency(selectedExpenseForFlag.amount)}</div>
                <div className="text-sm text-muted-foreground">
                  {selectedExpenseForFlag.description || selectedExpenseForFlag.vendor || 'No description'}
                </div>
                <div className="text-xs text-muted-foreground mt-1">
                  {new Date(selectedExpenseForFlag.date).toLocaleDateString()}
                </div>
              </div>
              {selectedExpenseForFlag.vendor && (
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    id="flagVendorFuture"
                    checked={flagVendorFuture}
                    onChange={(e) => setFlagVendorFuture(e.target.checked)}
                    className="rounded border-border"
                  />
                  <label htmlFor="flagVendorFuture" className="text-sm cursor-pointer">
                    Flag all future transactions from "{selectedExpenseForFlag.vendor}" for review
                  </label>
                </div>
              )}
            </div>
          )}
          <DialogFooter>
            <Button variant="ghost" onClick={() => setFlagDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={confirmFlagTransaction}>Flag Transaction</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </MainLayout>
  );
}

