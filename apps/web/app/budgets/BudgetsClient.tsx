'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createClient } from '@/lib/supabase/client';
import MainLayout from '../components/MainLayout';
import { useHousehold } from '../components/HouseholdProvider';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Button from '../components/ui/Button';
import Input from '../components/ui/Input';
import Select from '../components/ui/Select';
import {
  formatCurrency,
  getMonthRange,
  getYearRange,
  getCategoryColor,
  normalizeCategoryGroupColors,
} from '@twocents/shared';
import { Area, AreaChart, CartesianGrid, Legend, Pie, PieChart, ResponsiveContainer, Tooltip, XAxis, YAxis, Cell } from 'recharts';
import { ChevronDown, ChevronRight, RotateCcw } from 'lucide-react';

type Period = 'monthly' | 'yearly';
type Tab = 'main' | 'scenarios';

function isExcludedFromSpending(
  groupName: string | null | undefined, 
  category: { name?: string | null; exclude_from_calculations?: boolean; is_income_category?: boolean } | null | undefined
) {
  // Check the exclude_from_calculations field first
  if (category?.exclude_from_calculations) return true;
  
  // Income categories are excluded from spending calculations
  if (category?.is_income_category) return true;
  
  if (!groupName) return false;
  if (groupName === 'Savings & Investments') return true;
  if (groupName === 'Transfers') return true;
  if (groupName.startsWith('Income')) return true;
  return false;
}

function buildProjectedSavingsSeries(monthlyDelta: number, years: number, annualReturnPercent: number) {
  const months = Math.max(0, Math.round(years * 12));
  const monthlyRate = Math.max(0, annualReturnPercent) / 100 / 12;

  let balance = 0;
  const data: Array<{ month: number; projected: number }> = [];
  for (let m = 1; m <= months; m++) {
    balance = balance * (1 + monthlyRate) + monthlyDelta;
    data.push({ month: m, projected: balance });
  }
  return data;
}

export default function BudgetsClient() {
  const supabase = useMemo(() => createClient(), []);
  const { selectedHouseholdId, loading: householdsLoading } = useHousehold();

  const [activeTab, setActiveTab] = useState<Tab>('main');
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [categories, setCategories] = useState<any[]>([]);
  const [expenses, setExpenses] = useState<any[]>([]);
  const [mainBudgetSetId, setMainBudgetSetId] = useState<string | null>(null);
  const [existingBudgets, setExistingBudgets] = useState<any[]>([]);

  const [draftByCategoryId, setDraftByCategoryId] = useState<Record<string, string>>({});
  const [dirty, setDirty] = useState(false);
  const mainEditRevisionRef = useRef(0);
  const [mainLastSavedAt, setMainLastSavedAt] = useState<number | null>(null);

  const [scenarioSets, setScenarioSets] = useState<any[]>([]);
  const [selectedScenarioId, setSelectedScenarioId] = useState<string | null>(null);
  const [scenarioLoading, setScenarioLoading] = useState(false);
  const [scenarioSaving, setScenarioSaving] = useState(false);
  const [creatingScenario, setCreatingScenario] = useState(false);
  const [newScenarioName, setNewScenarioName] = useState('');
  const [copyFromMain, setCopyFromMain] = useState(true);
  const [scenarioBudgets, setScenarioBudgets] = useState<any[]>([]);
  const [scenarioDraftByCategoryId, setScenarioDraftByCategoryId] = useState<Record<string, string>>({});
  const [scenarioDirty, setScenarioDirty] = useState(false);
  const scenarioEditRevisionRef = useRef(0);
  const [scenarioLastSavedAt, setScenarioLastSavedAt] = useState<number | null>(null);

  const [baselineMonthlySpending, setBaselineMonthlySpending] = useState<number | null>(null);
  const [baselineLoading, setBaselineLoading] = useState(false);
  const [forecastYears, setForecastYears] = useState(5);
  const [forecastAnnualReturn, setForecastAnnualReturn] = useState(0);

  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(() => new Set());

  const [householdMembers, setHouseholdMembers] = useState<any[]>([]);
  const [householdIncome, setHouseholdIncome] = useState<Record<string, number>>({});
  const [incomeLoading, setIncomeLoading] = useState(false);
  const [incomeSaving, setIncomeSaving] = useState(false);
  const [incomeDirty, setIncomeDirty] = useState(false);
  const incomeEditRevisionRef = useRef(0);
  const [incomeLastSavedAt, setIncomeLastSavedAt] = useState<number | null>(null);

  const currentYear = useMemo(() => new Date().getFullYear(), []);

  const fetchHouseholdMembers = useCallback(async () => {
    if (!selectedHouseholdId) {
      setHouseholdMembers([]);
      return;
    }

    try {
      const { data, error } = await supabase
        .from('household_members')
        .select(
          `
          user_id,
          profiles:user_id (
            id,
            name
          )
        `
        )
        .eq('household_id', selectedHouseholdId);

      if (error) {
        console.warn('Error with relationship query, trying alternative:', error);
        const { data: membersData, error: membersError } = await supabase
          .from('household_members')
          .select('user_id')
          .eq('household_id', selectedHouseholdId);

        if (membersError || !membersData) {
          setHouseholdMembers([]);
          return;
        }

        const userIds = membersData.map((m: any) => m.user_id);
        const { data: profilesData, error: profilesError } = await supabase
          .from('profiles')
          .select('id, name')
          .in('id', userIds);

        if (profilesError) {
          console.warn('Error fetching profiles for household members:', profilesError);
        }
        const profileMap = new Map((profilesData || []).map((p: any) => [p.id, p]));

        setHouseholdMembers(
          membersData.map((member: any) => ({
            user_id: member.user_id,
            profiles: profileMap.get(member.user_id) || null,
          }))
        );
        return;
      }

      const normalized = (data || []).map((member: any) => {
        const profile = Array.isArray(member.profiles) ? member.profiles[0] : member.profiles;
        return {
          user_id: member.user_id,
          profiles: profile,
        };
      });

      setHouseholdMembers(normalized);
    } catch (err) {
      console.error('Error fetching household members:', err);
      setHouseholdMembers([]);
    }
  }, [selectedHouseholdId, supabase]);

  const fetchHouseholdIncome = useCallback(async () => {
    if (!selectedHouseholdId) {
      setHouseholdIncome({});
      return;
    }

    setIncomeLoading(true);
    try {
      const { data, error } = await supabase
        .from('household_income')
        .select('user_id, annual_income')
        .eq('household_id', selectedHouseholdId)
        .eq('year', currentYear);

      if (error) throw error;

      const incomeMap: Record<string, number> = {};
      (data || []).forEach((row: any) => {
        incomeMap[String(row.user_id)] = Number(row.annual_income || 0);
      });

      setHouseholdIncome(incomeMap);
      setIncomeDirty(false);
    } catch (err: any) {
      console.error('Error fetching household income:', err);
      setHouseholdIncome({});
    } finally {
      setIncomeLoading(false);
    }
  }, [selectedHouseholdId, currentYear, supabase]);

  const saveHouseholdIncome = useCallback(async () => {
    if (!selectedHouseholdId) return;
    const startRevision = incomeEditRevisionRef.current;
    setIncomeSaving(true);
    setError(null);

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) throw new Error('Not authenticated');

      const rows: any[] = [];
      for (const [userId, annualIncome] of Object.entries(householdIncome)) {
        const amount = Number(annualIncome) || 0;
        if (amount < 0) continue;

        rows.push({
          household_id: selectedHouseholdId,
          user_id: userId,
          annual_income: amount,
          year: currentYear,
        });
      }

      if (rows.length === 0) {
        // Delete all income records for this household/year if no income set
        await supabase
          .from('household_income')
          .delete()
          .eq('household_id', selectedHouseholdId)
          .eq('year', currentYear);
      } else {
        const { error: upsertError } = await supabase
          .from('household_income')
          .upsert(rows, { onConflict: 'household_id,user_id,year' });

        if (upsertError) throw upsertError;
      }

      if (incomeEditRevisionRef.current === startRevision) {
        setIncomeDirty(false);
        setIncomeLastSavedAt(Date.now());
      }
    } catch (e: any) {
      setError(e?.message ?? 'Failed to save household income');
    } finally {
      setIncomeSaving(false);
    }
  }, [householdIncome, selectedHouseholdId, currentYear, supabase]);

  const totalHouseholdIncome = useMemo(() => {
    return Object.values(householdIncome).reduce((sum, income) => sum + (Number(income) || 0), 0);
  }, [householdIncome]);

  const getOrCreateMainBudgetSetId = useCallback(async (): Promise<string | null> => {
    if (!selectedHouseholdId) return null;

    const { data: existing, error: existingError } = await supabase
      .from('budget_sets')
      .select('id')
      .eq('household_id', selectedHouseholdId)
      .eq('is_main', true)
      .maybeSingle();

    if (!existingError && existing?.id) return existing.id;

    const {
      data: { user },
    } = await supabase.auth.getUser();

    const { data: created, error: createError } = await supabase
      .from('budget_sets')
      .insert({
        household_id: selectedHouseholdId,
        name: 'Main Budget',
        is_main: true,
        created_by: user?.id ?? null,
      })
      .select('id')
      .single();

    if (!createError && created?.id) return created.id;

    // If insert conflicted (or failed), retry fetch
    const { data: retried } = await supabase
      .from('budget_sets')
      .select('id')
      .eq('household_id', selectedHouseholdId)
      .eq('is_main', true)
      .maybeSingle();

    return retried?.id ?? null;
  }, [selectedHouseholdId, supabase]);

  const fetchCategories = useCallback(async () => {
    if (!selectedHouseholdId) return;
    const { data: householdCats, error: householdError } = await supabase
      .from('categories')
      .select('*')
      .eq('household_id', selectedHouseholdId)
      .neq('is_hidden', true)
      .order('group_name')
      .order('name');

    if (householdError) throw householdError;

    let overrides: Record<string, string> | undefined;
    const { data: overrideRows, error: overrideError } = await supabase
      .from('household_category_group_colors')
      .select('group_name, color')
      .eq('household_id', selectedHouseholdId);

    if (!overrideError && overrideRows) {
      overrides = Object.fromEntries(overrideRows.map((row: any) => [String(row.group_name), String(row.color)]));
    }

    const normalized = normalizeCategoryGroupColors((householdCats || []) as any, overrides) as any[];
    setCategories(normalized);
  }, [selectedHouseholdId, supabase]);

  const fetchExpenses = useCallback(async () => {
    if (!selectedHouseholdId) return;
    const now = new Date();
    const yearRange = getYearRange(now);
    const yearStartIso = yearRange.start.toISOString().split('T')[0];
    const todayIso = now.toISOString().split('T')[0];
    const { data, error } = await supabase
      .from('expenses')
      .select(
        `
        id,
        amount,
        category_id,
        date,
        categories (
          id,
          group_name,
          name,
          exclude_from_calculations,
          is_income_category
        )
      `
      )
      .eq('household_id', selectedHouseholdId)
      .gte('date', yearStartIso)
      .lte('date', todayIso);
    if (error) throw error;
    setExpenses(data || []);
  }, [selectedHouseholdId, supabase]);

  const fetchBudgets = useCallback(
    async (budgetSetId: string) => {
      const { data, error } = await supabase
        .from('budgets')
        .select('id, category_id, amount, period, budget_set_id')
        .eq('budget_set_id', budgetSetId)
        .in('period', ['monthly', 'yearly']);
      if (error) throw error;
      setExistingBudgets(data || []);
    },
    [supabase]
  );

  const fetchScenarioBudgets = useCallback(
    async (budgetSetId: string) => {
      const { data, error } = await supabase
        .from('budgets')
        .select('id, category_id, amount, period, budget_set_id')
        .eq('budget_set_id', budgetSetId)
        .in('period', ['monthly', 'yearly']);
      if (error) throw error;
      setScenarioBudgets(data || []);
    },
    [supabase]
  );

  const fetchScenarioSets = useCallback(async () => {
    if (!selectedHouseholdId) return;
    const { data, error } = await supabase
      .from('budget_sets')
      .select('id, name, created_at')
      .eq('household_id', selectedHouseholdId)
      .eq('is_main', false)
      .order('created_at', { ascending: false });

    if (error) throw error;
    setScenarioSets(data || []);
  }, [selectedHouseholdId, supabase]);

  // Load everything
  useEffect(() => {
    if (!selectedHouseholdId) return;
    let cancelled = false;

    const run = async () => {
      setLoading(true);
      setError(null);
      try {
        const budgetSetId = await getOrCreateMainBudgetSetId();
        if (cancelled) return;
        setMainBudgetSetId(budgetSetId);

        await Promise.all([
          fetchCategories(),
          fetchExpenses(),
          budgetSetId ? fetchBudgets(budgetSetId) : Promise.resolve(),
          fetchScenarioSets(),
          fetchHouseholdMembers(),
        ]);
        if (cancelled) return;
        setDirty(false);
      } catch (e: any) {
        if (!cancelled) setError(e?.message ?? 'Failed to load budgets');
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    void run();
    return () => {
      cancelled = true;
    };
  }, [fetchBudgets, fetchCategories, fetchExpenses, fetchScenarioSets, fetchHouseholdMembers, getOrCreateMainBudgetSetId, selectedHouseholdId]);

  // Load household income
  useEffect(() => {
    if (!selectedHouseholdId) {
      setHouseholdIncome({});
      return;
    }
    void fetchHouseholdIncome();
  }, [fetchHouseholdIncome, selectedHouseholdId]);

  // Baseline monthly spending from recent behavior (last ~90 days), used for forecasting
  useEffect(() => {
    if (!selectedHouseholdId) {
      setBaselineMonthlySpending(null);
      return;
    }

    let cancelled = false;

    const run = async () => {
      setBaselineLoading(true);
      try {
        const end = new Date();
        const start = new Date();
        start.setDate(end.getDate() - 90);

        const startIso = start.toISOString().split('T')[0];
        const endIso = end.toISOString().split('T')[0];

        const { data, error } = await supabase
          .from('expenses')
          .select(
            `
            amount,
            date,
            categories (
              group_name,
              name,
              exclude_from_calculations,
              is_income_category
            )
          `
          )
          .eq('household_id', selectedHouseholdId)
          .gte('date', startIso)
          .lte('date', endIso);

        if (error) throw error;

        const sum = (data || []).reduce((acc: number, e: any) => {
          const groupName = e?.categories?.group_name ?? null;
          const category = e?.categories ?? null;
          if (isExcludedFromSpending(groupName, category)) return acc;
          return acc + Number(e.amount ?? 0);
        }, 0);

        const days = Math.max(1, (end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));
        const months = Math.max(1, days / 30);
        const baseline = sum / months;

        if (!cancelled) setBaselineMonthlySpending(baseline);
      } catch {
        if (!cancelled) setBaselineMonthlySpending(null);
      } finally {
        if (!cancelled) setBaselineLoading(false);
      }
    };

    void run();
    return () => {
      cancelled = true;
    };
  }, [selectedHouseholdId, supabase]);

  // Load budgets for selected scenario
  useEffect(() => {
    if (!selectedScenarioId) {
      setScenarioBudgets([]);
      setScenarioDraftByCategoryId({});
      setScenarioDirty(false);
      return;
    }

    let cancelled = false;

    const run = async () => {
      setScenarioLoading(true);
      setError(null);
      try {
        await fetchScenarioBudgets(selectedScenarioId);
        if (cancelled) return;
        setScenarioDirty(false);
      } catch (e: any) {
        if (!cancelled) setError(e?.message ?? 'Failed to load scenario budgets');
      } finally {
        if (!cancelled) setScenarioLoading(false);
      }
    };

    void run();
    return () => {
      cancelled = true;
    };
  }, [fetchScenarioBudgets, selectedScenarioId]);

  // Initialize draft values from fetched budgets/categories
  useEffect(() => {
    if (!selectedHouseholdId) return;
    if (dirty) return;

    const map: Record<string, string> = {};
    const monthlyByCategoryId = new Map<string, number>();
    const yearlyByCategoryId = new Map<string, number>();
    for (const b of existingBudgets) {
      if (!b?.category_id) continue;
      const id = String(b.category_id);
      const amt = Number(b.amount ?? 0);
      if (b.period === 'monthly') {
        monthlyByCategoryId.set(id, amt);
      }
      if (b.period === 'yearly') {
        yearlyByCategoryId.set(id, amt);
      }
    }

    for (const c of categories) {
      const id = String(c.id);
      const monthly = monthlyByCategoryId.get(id);
      if (monthly !== undefined) {
        map[id] = String(monthly);
        continue;
      }
      const yearly = yearlyByCategoryId.get(id);
      if (yearly !== undefined) {
        map[id] = String(yearly / 12);
        continue;
      }
      map[id] = '';
    }

    setDraftByCategoryId(map);
  }, [categories, dirty, existingBudgets, selectedHouseholdId]);

  // Initialize scenario draft values from fetched budgets/categories
  useEffect(() => {
    if (!selectedHouseholdId || !selectedScenarioId) return;
    if (scenarioDirty) return;

    const map: Record<string, string> = {};
    const monthlyByCategoryId = new Map<string, number>();
    const yearlyByCategoryId = new Map<string, number>();
    for (const b of scenarioBudgets) {
      if (!b?.category_id) continue;
      const id = String(b.category_id);
      const amt = Number(b.amount ?? 0);
      if (b.period === 'monthly') {
        monthlyByCategoryId.set(id, amt);
      }
      if (b.period === 'yearly') {
        yearlyByCategoryId.set(id, amt);
      }
    }

    for (const c of categories) {
      const id = String(c.id);
      const monthly = monthlyByCategoryId.get(id);
      if (monthly !== undefined) {
        map[id] = String(monthly);
        continue;
      }
      const yearly = yearlyByCategoryId.get(id);
      if (yearly !== undefined) {
        map[id] = String(yearly / 12);
        continue;
      }
      map[id] = '';
    }

    setScenarioDraftByCategoryId(map);
  }, [categories, scenarioBudgets, scenarioDirty, selectedHouseholdId, selectedScenarioId]);

  const monthStartIso = useMemo(() => getMonthRange().start.toISOString().split('T')[0], []);
  const monthsElapsed = useMemo(() => new Date().getMonth() + 1, []);

  const actualYtdByCategoryId = useMemo(() => {
    const map = new Map<string, number>();
    for (const e of expenses) {
      const key = e.category_id ? String(e.category_id) : 'uncategorized';
      map.set(key, (map.get(key) || 0) + Number(e.amount ?? 0));
    }
    return map;
  }, [expenses]);

  const actualMonthByCategoryId = useMemo(() => {
    const map = new Map<string, number>();
    for (const e of expenses) {
      const date = String(e.date || '');
      if (!date || date < monthStartIso) continue;
      const key = e.category_id ? String(e.category_id) : 'uncategorized';
      map.set(key, (map.get(key) || 0) + Number(e.amount ?? 0));
    }
    return map;
  }, [expenses, monthStartIso]);

  const categoryById = useMemo(() => {
    const map = new Map<string, any>();
    for (const c of categories) map.set(String(c.id), c);
    return map;
  }, [categories]);

  const displayCategories = useMemo(() => {
    const pseudo = {
      id: 'uncategorized',
      name: 'Uncategorized',
      group_name: 'Uncategorized',
      color: '#95A5A6',
      parent_color: null,
    };
    return [pseudo, ...categories];
  }, [categories]);

  const groups = useMemo(() => {
    const grouped = new Map<string, any[]>();
    for (const c of displayCategories) {
      const groupName = String(c.group_name || 'Other');
      if (!grouped.has(groupName)) grouped.set(groupName, []);
      grouped.get(groupName)!.push(c);
    }

    const sortedGroups = Array.from(grouped.entries())
      .map(([groupName, cats]) => ({
        groupName,
        categories: cats.sort((a, b) => String(a.name).localeCompare(String(b.name))),
      }))
      .sort((a, b) => a.groupName.localeCompare(b.groupName));

    return sortedGroups;
  }, [displayCategories]);

  const parsedBudgetByCategoryId = useMemo(() => {
    const map = new Map<string, number>();
    for (const [categoryId, raw] of Object.entries(draftByCategoryId)) {
      const num = raw === '' ? 0 : Number(raw);
      map.set(categoryId, Number.isFinite(num) ? num : 0);
    }
    return map;
  }, [draftByCategoryId]);

  const budgetedSpending = useMemo(() => {
    let sum = 0;
    for (const [categoryId, amount] of parsedBudgetByCategoryId.entries()) {
      if (categoryId === 'uncategorized') continue;
      const category = categoryById.get(categoryId);
      const groupName = category?.group_name ?? null;
      if (isExcludedFromSpending(groupName, category)) continue;
      sum += amount;
    }
    return sum;
  }, [categoryById, parsedBudgetByCategoryId]);

  const budgetedSavings = useMemo(() => {
    let sum = 0;
    for (const [categoryId, amount] of parsedBudgetByCategoryId.entries()) {
      if (categoryId === 'uncategorized') continue;
      const groupName = categoryById.get(categoryId)?.group_name ?? null;
      if (groupName !== 'Savings & Investments') continue;
      sum += amount;
    }
    return sum;
  }, [categoryById, parsedBudgetByCategoryId]);

  const parsedScenarioBudgetByCategoryId = useMemo(() => {
    const map = new Map<string, number>();
    for (const [categoryId, raw] of Object.entries(scenarioDraftByCategoryId)) {
      const num = raw === '' ? 0 : Number(raw);
      map.set(categoryId, Number.isFinite(num) ? num : 0);
    }
    return map;
  }, [scenarioDraftByCategoryId]);

  const toggleGroup = useCallback((groupName: string) => {
    setExpandedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(groupName)) next.delete(groupName);
      else next.add(groupName);
      return next;
    });
  }, []);

  const scenarioBudgetedSpending = useMemo(() => {
    let sum = 0;
    for (const [categoryId, amount] of parsedScenarioBudgetByCategoryId.entries()) {
      const category = categoryById.get(categoryId);
      const groupName = category?.group_name ?? null;
      if (isExcludedFromSpending(groupName, category)) continue;
      sum += amount;
    }
    return sum;
  }, [categoryById, parsedScenarioBudgetByCategoryId]);

  const scenarioBudgetedSavings = useMemo(() => {
    let sum = 0;
    for (const [categoryId, amount] of parsedScenarioBudgetByCategoryId.entries()) {
      const groupName = categoryById.get(categoryId)?.group_name ?? null;
      if (groupName !== 'Savings & Investments') continue;
      sum += amount;
    }
    return sum;
  }, [categoryById, parsedScenarioBudgetByCategoryId]);

  const plannedMonthlySpendingMain = useMemo(() => budgetedSpending, [budgetedSpending]);
  const plannedMonthlySpendingScenario = useMemo(() => scenarioBudgetedSpending, [scenarioBudgetedSpending]);

  const actualSpending = useMemo(() => {
    let sum = 0;
    for (const e of expenses) {
      const date = String(e.date || '');
      if (!date || date < monthStartIso) continue;
      const groupName = e?.categories?.group_name ?? null;
      const category = e?.categories ?? null;
      if (isExcludedFromSpending(groupName, category)) continue;
      sum += Number(e.amount ?? 0);
    }
    return sum;
  }, [expenses, monthStartIso]);

  const actualSavings = useMemo(() => {
    return expenses
      .filter((e: any) => String(e?.date || '') >= monthStartIso && e?.categories?.group_name === 'Savings & Investments')
      .reduce((sum: number, e: any) => sum + Number(e.amount ?? 0), 0);
  }, [expenses, monthStartIso]);

  // Calculate actual income from transactions
  const actualIncome = useMemo(() => {
    let sum = 0;
    for (const e of expenses) {
      const date = String(e.date || '');
      if (!date || date < monthStartIso) continue;
      const category = e?.categories ?? null;
      // Only count positive amounts in income categories
      if (category?.is_income_category && Number(e.amount) > 0) {
        sum += Number(e.amount);
      }
    }
    return sum;
  }, [expenses, monthStartIso]);

  // Calculate projected monthly income from household_income table
  const projectedMonthlyIncome = useMemo(() => {
    return totalHouseholdIncome / 12;
  }, [totalHouseholdIncome]);

  // Calculate income difference
  const incomeDifference = useMemo(() => {
    return actualIncome - projectedMonthlyIncome;
  }, [actualIncome, projectedMonthlyIncome]);

  const handleChangeBudget = useCallback((categoryId: string, value: string) => {
    setDraftByCategoryId((prev) => {
      const next = { ...prev, [categoryId]: value };
      return next;
    });
    mainEditRevisionRef.current += 1;
    setDirty(true);
  }, []);

  const handleResetBudgets = useCallback(() => {
    if (!window.confirm('Are you sure you want to clear all budgets? This action cannot be undone.')) {
      return;
    }
    
    // Clear all budget values
    const cleared: Record<string, string> = {};
    for (const c of categories) {
      cleared[String(c.id)] = '';
    }
    setDraftByCategoryId(cleared);
    mainEditRevisionRef.current += 1;
    setDirty(true);
  }, [categories]);

  const handleChangeScenarioBudget = useCallback((categoryId: string, value: string) => {
    setScenarioDraftByCategoryId((prev) => ({ ...prev, [categoryId]: value }));
    scenarioEditRevisionRef.current += 1;
    setScenarioDirty(true);
  }, []);

  const handleSave = useCallback(async () => {
    if (!selectedHouseholdId || !mainBudgetSetId) return;
    const startRevision = mainEditRevisionRef.current;
    setSaving(true);
    setError(null);
    try {
      const existingSet = new Set(existingBudgets.map((b) => String(b.category_id)));
      const existingMonthlyByCategoryId = new Map<string, number>();
      const existingYearlyByCategoryId = new Map<string, number>();
      for (const b of existingBudgets) {
        const id = String(b.category_id);
        const amt = Number(b.amount ?? 0);
        if (b.period === 'monthly') {
          existingMonthlyByCategoryId.set(id, amt);
        }
        if (b.period === 'yearly') {
          existingYearlyByCategoryId.set(id, amt);
        }
      }

      const rows: any[] = [];
      for (const c of categories) {
        const categoryId = String(c.id);
        const raw = draftByCategoryId[categoryId] ?? '';
        const amount = raw === '' ? 0 : Number(raw);
        const normalized = Number.isFinite(amount) ? Math.max(0, amount) : 0;

        const existed = existingSet.has(categoryId);
        if (normalized <= 0 && !existed) continue;

        const existingMonthly =
          existingMonthlyByCategoryId.get(categoryId) ??
          (existingYearlyByCategoryId.get(categoryId) !== undefined
            ? (existingYearlyByCategoryId.get(categoryId) as number) / 12
            : 0);

        // If unchanged, skip writing to reduce load (important with autosave)
        if (Math.abs(existingMonthly - normalized) < 0.0001) continue;

        rows.push({
          household_id: selectedHouseholdId,
          budget_set_id: mainBudgetSetId,
          category_id: categoryId,
          amount: normalized,
          period: 'monthly',
        });

        rows.push({
          household_id: selectedHouseholdId,
          budget_set_id: mainBudgetSetId,
          category_id: categoryId,
          amount: normalized * 12,
          period: 'yearly',
        });
      }

      if (rows.length === 0) {
        if (mainEditRevisionRef.current === startRevision) {
          setDirty(false);
          setMainLastSavedAt(Date.now());
        }
        return;
      }

      const { error: upsertError } = await supabase
        .from('budgets')
        .upsert(rows, { onConflict: 'budget_set_id,category_id,period' });

      if (upsertError) throw upsertError;

      // Update local cache without refetching (avoid clobbering drafts mid-edit)
      setExistingBudgets((prev) => {
        const next = [...prev];
        for (const row of rows) {
          const idx = next.findIndex(
            (b: any) =>
              String(b.category_id) === String(row.category_id) &&
              String(b.budget_set_id) === String(row.budget_set_id) &&
              b.period === row.period
          );
          if (idx >= 0) {
            next[idx] = { ...next[idx], amount: row.amount };
          } else {
            next.push({
              id: '',
              category_id: row.category_id,
              amount: row.amount,
              period: row.period,
              budget_set_id: row.budget_set_id,
            });
          }
        }
        return next;
      });

      if (mainEditRevisionRef.current === startRevision) {
        setDirty(false);
        setMainLastSavedAt(Date.now());
      }
    } catch (e: any) {
      setError(e?.message ?? 'Failed to save budgets');
    } finally {
      setSaving(false);
    }
  }, [categories, draftByCategoryId, existingBudgets, mainBudgetSetId, selectedHouseholdId, supabase]);

  const handleSaveScenario = useCallback(async () => {
    if (!selectedHouseholdId || !selectedScenarioId) return;
    const startRevision = scenarioEditRevisionRef.current;
    setScenarioSaving(true);
    setError(null);
    try {
      const existingSet = new Set(scenarioBudgets.map((b) => String(b.category_id)));
      const existingMonthlyByCategoryId = new Map<string, number>();
      const existingYearlyByCategoryId = new Map<string, number>();
      for (const b of scenarioBudgets) {
        const id = String(b.category_id);
        const amt = Number(b.amount ?? 0);
        if (b.period === 'monthly') {
          existingMonthlyByCategoryId.set(id, amt);
        }
        if (b.period === 'yearly') {
          existingYearlyByCategoryId.set(id, amt);
        }
      }

      const rows: any[] = [];
      for (const c of categories) {
        const categoryId = String(c.id);
        const raw = scenarioDraftByCategoryId[categoryId] ?? '';
        const amount = raw === '' ? 0 : Number(raw);
        const normalized = Number.isFinite(amount) ? Math.max(0, amount) : 0;

        const existed = existingSet.has(categoryId);
        if (normalized <= 0 && !existed) continue;

        const existingMonthly =
          existingMonthlyByCategoryId.get(categoryId) ??
          (existingYearlyByCategoryId.get(categoryId) !== undefined
            ? (existingYearlyByCategoryId.get(categoryId) as number) / 12
            : 0);

        if (Math.abs(existingMonthly - normalized) < 0.0001) continue;

        rows.push({
          household_id: selectedHouseholdId,
          budget_set_id: selectedScenarioId,
          category_id: categoryId,
          amount: normalized,
          period: 'monthly',
        });
        rows.push({
          household_id: selectedHouseholdId,
          budget_set_id: selectedScenarioId,
          category_id: categoryId,
          amount: normalized * 12,
          period: 'yearly',
        });
      }

      if (rows.length === 0) {
        if (scenarioEditRevisionRef.current === startRevision) {
          setScenarioDirty(false);
          setScenarioLastSavedAt(Date.now());
        }
        return;
      }

      const { error: upsertError } = await supabase
        .from('budgets')
        .upsert(rows, { onConflict: 'budget_set_id,category_id,period' });

      if (upsertError) throw upsertError;

      setScenarioBudgets((prev) => {
        const next = [...prev];
        for (const row of rows) {
          const idx = next.findIndex(
            (b: any) =>
              String(b.category_id) === String(row.category_id) &&
              String(b.budget_set_id) === String(row.budget_set_id) &&
              b.period === row.period
          );
          if (idx >= 0) {
            next[idx] = { ...next[idx], amount: row.amount };
          } else {
            next.push({
              id: '',
              category_id: row.category_id,
              amount: row.amount,
              period: row.period,
              budget_set_id: row.budget_set_id,
            });
          }
        }
        return next;
      });

      if (scenarioEditRevisionRef.current === startRevision) {
        setScenarioDirty(false);
        setScenarioLastSavedAt(Date.now());
      }
    } catch (e: any) {
      setError(e?.message ?? 'Failed to save scenario budgets');
    } finally {
      setScenarioSaving(false);
    }
  }, [
    categories,
    scenarioBudgets,
    scenarioDraftByCategoryId,
    selectedHouseholdId,
    selectedScenarioId,
    supabase,
  ]);

  const handleCreateScenario = useCallback(async () => {
    if (!selectedHouseholdId) return;
    const name = newScenarioName.trim();
    if (!name) return;

    setCreatingScenario(true);
    setError(null);
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();

      const { data: created, error: createError } = await supabase
        .from('budget_sets')
        .insert({
          household_id: selectedHouseholdId,
          name,
          is_main: false,
          created_by: user?.id ?? null,
        })
        .select('id, name')
        .single();

      if (createError) throw createError;
      if (!created?.id) throw new Error('Failed to create scenario');

      if (copyFromMain && mainBudgetSetId) {
        const { data: mainBudgets, error: mainBudgetsError } = await supabase
          .from('budgets')
          .select('category_id, amount, period')
          .eq('budget_set_id', mainBudgetSetId);

        if (mainBudgetsError) throw mainBudgetsError;

        const rows =
          (mainBudgets || []).map((b: any) => ({
            household_id: selectedHouseholdId,
            budget_set_id: created.id,
            category_id: b.category_id,
            amount: b.amount,
            period: b.period,
          })) ?? [];

        if (rows.length > 0) {
          const { error: copyError } = await supabase.from('budgets').insert(rows);
          if (copyError) throw copyError;
        }
      }

      await fetchScenarioSets();
      setNewScenarioName('');
      setSelectedScenarioId(created.id);
      setActiveTab('scenarios');
    } catch (e: any) {
      setError(e?.message ?? 'Failed to create scenario');
    } finally {
      setCreatingScenario(false);
    }
  }, [copyFromMain, fetchScenarioSets, mainBudgetSetId, newScenarioName, selectedHouseholdId, supabase]);

  const handleDeleteScenario = useCallback(
    async (scenarioId: string) => {
      if (!scenarioId) return;
      setError(null);
      try {
        const { error: deleteError } = await supabase.from('budget_sets').delete().eq('id', scenarioId);
        if (deleteError) throw deleteError;
        if (selectedScenarioId === scenarioId) {
          setSelectedScenarioId(null);
          setScenarioBudgets([]);
          setScenarioDraftByCategoryId({});
          setScenarioDirty(false);
        }
        await fetchScenarioSets();
      } catch (e: any) {
        setError(e?.message ?? 'Failed to delete scenario');
      }
    },
    [fetchScenarioSets, selectedScenarioId, supabase]
  );

  // Autosave (debounced)
  useEffect(() => {
    if (!dirty) return;
    if (saving) return;
    if (!selectedHouseholdId || !mainBudgetSetId) return;
    const t = window.setTimeout(() => {
      void handleSave();
    }, 600);
    return () => window.clearTimeout(t);
  }, [dirty, draftByCategoryId, handleSave, mainBudgetSetId, saving, selectedHouseholdId]);

  useEffect(() => {
    if (!scenarioDirty) return;
    if (scenarioSaving) return;
    if (!selectedHouseholdId || !selectedScenarioId) return;
    const t = window.setTimeout(() => {
      void handleSaveScenario();
    }, 600);
    return () => window.clearTimeout(t);
  }, [handleSaveScenario, scenarioDirty, scenarioDraftByCategoryId, scenarioSaving, selectedHouseholdId, selectedScenarioId]);

  // Removed autosave - using manual save button instead to prevent lag

  const isLoading = householdsLoading || (selectedHouseholdId ? loading : false);
  const selectedScenario = useMemo(() => {
    if (!selectedScenarioId) return null;
    return scenarioSets.find((s) => s.id === selectedScenarioId) ?? null;
  }, [scenarioSets, selectedScenarioId]);

  return (
    <MainLayout
      pageHeader={{
        title: 'Budgets',
        subtitle: 'Set spending and savings targets for your household',
      }}
      contentVariant="full-width"
      contentPaddingY="none"
    >
      {isLoading ? (
        <div className="flex items-center justify-center py-24">
          <div className="text-muted-foreground">Loading budgets...</div>
        </div>
      ) : !selectedHouseholdId ? (
        <Card>
          <CardContent className="py-8 text-center">
            <p className="text-muted-foreground">Please select or create a household first.</p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-6">
          {error && (
            <Card>
              <CardContent className="py-4 text-sm text-red-600">{error}</CardContent>
            </Card>
          )}

          {/* Tabs + autosave status */}
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <Button
                variant={activeTab === 'main' ? 'secondary' : 'ghost'}
                onClick={() => setActiveTab('main')}
              >
                Main Budget
              </Button>
              <Button
                variant={activeTab === 'scenarios' ? 'secondary' : 'ghost'}
                onClick={() => setActiveTab('scenarios')}
              >
                Scenarios
              </Button>
            </div>
            <div className="text-xs text-muted-foreground">
              {activeTab === 'main' ? (
                saving ? (
                  'Saving...'
                ) : dirty ? (
                  'Unsaved changes'
                ) : mainLastSavedAt ? (
                  'Saved'
                ) : (
                  ''
                )
              ) : !selectedScenarioId ? (
                ''
              ) : scenarioSaving ? (
                'Saving...'
              ) : scenarioDirty ? (
                'Unsaved changes'
              ) : scenarioLastSavedAt ? (
                'Saved'
              ) : (
                ''
              )}
            </div>
          </div>

          {/* Income and Summary - Side by side above budget editor */}
          {activeTab === 'main' && (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
              <IncomeInputCard
                householdMembers={householdMembers}
                householdIncome={householdIncome}
                incomeLoading={incomeLoading}
                incomeSaving={incomeSaving}
                incomeDirty={incomeDirty}
                incomeLastSavedAt={incomeLastSavedAt}
                totalHouseholdIncome={totalHouseholdIncome}
                onIncomeChange={(userId, value) => {
                  const numValue = value === '' ? 0 : parseFloat(value) || 0;
                  setHouseholdIncome((prev) => ({ ...prev, [userId]: numValue }));
                  setIncomeDirty(true);
                }}
                onSave={saveHouseholdIncome}
              />

              <CompactSummaryCard
                title="Summary"
                spending={{ budgetedMonthly: budgetedSpending, actualMonthly: actualSpending }}
                savings={{ budgetedMonthly: budgetedSavings, actualMonthly: actualSavings }}
              />
            </div>
          )}

          {/* Budget editor - Full width */}
          <div className="space-y-6">
            {activeTab === 'main' ? (
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <CardTitle className="text-base">Main Budget</CardTitle>
                      <CardDescription className="text-xs">Monthly editing, grouped by category</CardDescription>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setExpandedGroups(new Set(groups.map((g) => g.groupName)))}
                        type="button"
                      >
                        Expand all
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setExpandedGroups(new Set())}
                        type="button"
                      >
                        Collapse all
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={handleResetBudgets}
                        type="button"
                        className="text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-950"
                      >
                        <RotateCcw className="h-4 w-4 mr-1" />
                        Reset
                      </Button>
                    </div>
                  </div>
                </CardHeader>
                <CardContent className="pt-0">
                  <div className="overflow-x-auto relative">
                    <table className="w-full border-collapse">
                      <thead className="sticky top-0 z-40 bg-card">
                        <tr className="border-b border-border">
                          <th className="p-2 text-left text-xs font-semibold whitespace-nowrap sticky top-0 left-0 z-50 bg-card border-r border-border" style={{ minWidth: '200px', width: '200px' }}>
                            Category
                          </th>
                          <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Budget (Mo)</th>
                          <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Budget (Yr)</th>
                          <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Remaining</th>
                          <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Actual (Mo)</th>
                          <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Actual Avg</th>
                          <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Actual (YTD)</th>
                        </tr>
                      </thead>
                      <tbody>
                        {groups.map(({ groupName, categories: groupCats }) => (
                          <FragmentGroup
                            key={groupName}
                            groupName={groupName}
                            categories={groupCats}
                            draftByCategoryId={draftByCategoryId}
                            actualMonthByCategoryId={actualMonthByCategoryId}
                            actualYtdByCategoryId={actualYtdByCategoryId}
                            monthsElapsed={monthsElapsed}
                            expanded={expandedGroups.has(groupName)}
                            onToggle={() => toggleGroup(groupName)}
                            onChangeBudget={handleChangeBudget}
                          />
                        ))}
                      </tbody>
                    </table>
                  </div>
                </CardContent>
              </Card>
            ) : (
              <>
                <Card>
                  <CardHeader>
                    <CardTitle>Scenarios</CardTitle>
                    <CardDescription>Temporary budgets for planning</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div className="space-y-2">
                      <Input
                        value={newScenarioName}
                        onChange={(e) => setNewScenarioName(e.target.value)}
                        placeholder="Scenario name (e.g., Vacation 2026)"
                      />
                      <label className="flex items-center gap-2 text-sm text-muted-foreground">
                        <input
                          type="checkbox"
                          checked={copyFromMain}
                          onChange={(e) => setCopyFromMain(e.target.checked)}
                          className="h-4 w-4 rounded border-border"
                        />
                        Copy current main budgets into this scenario
                      </label>
                      <Button
                        onClick={handleCreateScenario}
                        disabled={creatingScenario || !newScenarioName.trim()}
                        className="w-full"
                      >
                        {creatingScenario ? 'Creating...' : 'Create scenario'}
                      </Button>
                    </div>

                    <div className="space-y-2">
                      {scenarioSets.length === 0 ? (
                        <div className="text-sm text-muted-foreground">No scenarios yet.</div>
                      ) : (
                        scenarioSets.map((s) => {
                          const isSelected = selectedScenarioId === s.id;
                          return (
                            <div
                              key={s.id}
                              className={`flex items-center justify-between gap-3 rounded-notion border border-border p-3 ${
                                isSelected ? 'bg-hover' : ''
                              }`}
                            >
                              <button
                                className="flex-1 text-left min-w-0"
                                onClick={() => setSelectedScenarioId(s.id)}
                                type="button"
                              >
                                <div className="truncate text-sm font-medium">{s.name}</div>
                                <div className="text-xs text-muted-foreground">Saved scenario</div>
                              </button>
                              <Button
                                variant="danger"
                                size="sm"
                                onClick={() => handleDeleteScenario(s.id)}
                                type="button"
                              >
                                Delete
                              </Button>
                            </div>
                          );
                        })
                      )}
                    </div>
                  </CardContent>
                </Card>

                {!selectedScenarioId ? (
                  <Card>
                    <CardContent className="py-8 text-center">
                      <p className="text-muted-foreground">Select a scenario to edit, or create a new one.</p>
                    </CardContent>
                  </Card>
                ) : scenarioLoading ? (
                  <Card>
                    <CardContent className="py-8 text-center">
                      <p className="text-muted-foreground">Loading scenario...</p>
                    </CardContent>
                  </Card>
                ) : (
                  <>
                    <Card>
                      <CardHeader>
                        <CardTitle>{selectedScenario?.name || 'Scenario budget'}</CardTitle>
                        <CardDescription>Adjust monthly budgets per category</CardDescription>
                      </CardHeader>
                      <CardContent className="pt-0">
                        <div className="overflow-x-auto relative">
                          <table className="w-full border-collapse">
                            <thead className="sticky top-0 z-40 bg-card">
                              <tr className="border-b border-border">
                                <th className="p-2 text-left text-xs font-semibold whitespace-nowrap sticky top-0 left-0 z-50 bg-card border-r border-border" style={{ minWidth: '200px', width: '200px' }}>
                                  Category
                                </th>
                                <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Budget (Mo)</th>
                                <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Budget (Yr)</th>
                                <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Remaining</th>
                                <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Actual (Mo)</th>
                                <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Actual Avg</th>
                                <th className="p-2 text-left text-xs font-semibold whitespace-nowrap bg-card">Actual (YTD)</th>
                              </tr>
                            </thead>
                            <tbody>
                              {groups.map(({ groupName, categories: groupCats }) => (
                                <FragmentGroup
                                  key={groupName}
                                  groupName={groupName}
                                  categories={groupCats}
                                  draftByCategoryId={scenarioDraftByCategoryId}
                                  actualMonthByCategoryId={actualMonthByCategoryId}
                                  actualYtdByCategoryId={actualYtdByCategoryId}
                                  monthsElapsed={monthsElapsed}
                                  expanded={expandedGroups.has(groupName)}
                                  onToggle={() => toggleGroup(groupName)}
                                  onChangeBudget={handleChangeScenarioBudget}
                                />
                              ))}
                            </tbody>
                          </table>
                        </div>
                      </CardContent>
                    </Card>
                    {selectedScenarioId && !scenarioLoading && (
                      <CompactSummaryCard
                        title={selectedScenario?.name || 'Scenario summary'}
                        spending={{ budgetedMonthly: scenarioBudgetedSpending, actualMonthly: actualSpending }}
                        savings={{ budgetedMonthly: scenarioBudgetedSavings, actualMonthly: actualSavings }}
                      />
                    )}
                  </>
                )}
              </>
            )}
          </div>

          {/* Savings Forecast - Full width under budget */}
          {activeTab === 'main' ? (
            <SavingsForecastCard
              title="Forecast: Projected Extra Savings"
              description="See how adjusting your budget could increase savings over time"
              baselineMonthlySpending={baselineMonthlySpending}
              baselineLoading={baselineLoading}
              plannedMonthlySpending={plannedMonthlySpendingMain}
              forecastYears={forecastYears}
              setForecastYears={setForecastYears}
              forecastAnnualReturn={forecastAnnualReturn}
              setForecastAnnualReturn={setForecastAnnualReturn}
            />
          ) : selectedScenarioId && !scenarioLoading ? (
            <SavingsForecastCard
              title="Forecast: Projected Extra Savings"
              description="Forecast is relative to your recent spending baseline"
              baselineMonthlySpending={baselineMonthlySpending}
              baselineLoading={baselineLoading}
              plannedMonthlySpending={plannedMonthlySpendingScenario}
              forecastYears={forecastYears}
              setForecastYears={setForecastYears}
              forecastAnnualReturn={forecastAnnualReturn}
              setForecastAnnualReturn={setForecastAnnualReturn}
            />
          ) : null}

          {/* Full-width charts section */}
          {activeTab === 'main' ? (
              <BudgetForecastCharts
                categories={categories}
                budgetedByCategoryId={parsedBudgetByCategoryId}
                actualByCategoryId={actualMonthByCategoryId}
                actualYtdByCategoryId={actualYtdByCategoryId}
                totalIncome={totalHouseholdIncome}
                monthsElapsed={monthsElapsed}
              />
            ) : selectedScenarioId && !scenarioLoading ? (
              <BudgetForecastCharts
                categories={categories}
                budgetedByCategoryId={parsedScenarioBudgetByCategoryId}
                actualByCategoryId={actualMonthByCategoryId}
                actualYtdByCategoryId={actualYtdByCategoryId}
                totalIncome={totalHouseholdIncome}
                monthsElapsed={monthsElapsed}
              />
            ) : null}
        </div>
      )}
    </MainLayout>
  );
}

function IncomeInputCard({
  householdMembers,
  householdIncome,
  incomeLoading,
  incomeSaving,
  incomeDirty,
  incomeLastSavedAt,
  totalHouseholdIncome,
  onIncomeChange,
  onSave,
}: {
  householdMembers: any[];
  householdIncome: Record<string, number>;
  incomeLoading: boolean;
  incomeSaving: boolean;
  incomeDirty: boolean;
  incomeLastSavedAt: number | null;
  totalHouseholdIncome: number;
  onIncomeChange: (userId: string, value: string) => void;
  onSave: () => Promise<void>;
}) {
  return (
    <Card>
      <CardHeader className="pb-1.5 pt-3 px-3">
        <div className="flex items-center justify-between gap-2">
          <div>
            <CardTitle className="text-xs font-semibold">Income</CardTitle>
            <CardDescription className="text-[9px] leading-tight">{new Date().getFullYear()}</CardDescription>
          </div>
          <div className="text-[9px] text-muted-foreground">
            {incomeSaving ? 'Saving...' : incomeDirty ? 'Unsaved' : incomeLastSavedAt ? 'Saved' : ''}
          </div>
        </div>
      </CardHeader>
      <CardContent className="pt-0 px-3 pb-3 space-y-1.5">
        {incomeLoading ? (
          <div className="text-[10px] text-muted-foreground">Loading...</div>
        ) : householdMembers.length === 0 ? (
          <div className="text-[10px] text-muted-foreground">No members</div>
        ) : (
          <>
            <div className="space-y-1">
              {householdMembers.map((member) => {
                const userId = String(member.user_id);
                const memberName = member.profiles?.name || 'Unknown';
                const currentIncome = householdIncome[userId] || '';

                return (
                  <div key={userId} className="flex items-center gap-1.5">
                    <label className="text-[10px] font-medium text-muted-foreground w-16 truncate">{memberName}</label>
                    <Input
                      type="number"
                      inputMode="decimal"
                      step="0.01"
                      min="0"
                      value={String(currentIncome)}
                      onChange={(e) => onIncomeChange(userId, e.target.value)}
                      placeholder="0"
                      className="flex-1 h-7 text-xs px-2 py-1"
                    />
                    <span className="text-[9px] text-muted-foreground whitespace-nowrap">/yr</span>
                  </div>
                );
              })}
            </div>
            <div className="pt-1.5 border-t border-border space-y-1">
              <div className="flex items-center justify-between">
                <span className="text-[9px] font-medium text-muted-foreground">Total</span>
                <span className="text-[11px] font-semibold">{formatCurrency(totalHouseholdIncome)}</span>
              </div>
              <Button
                onClick={() => void onSave()}
                disabled={incomeSaving || !incomeDirty}
                className="w-full h-7 text-[10px] px-2 py-0"
                size="sm"
              >
                {incomeSaving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}

function SavingsForecastCard({
  title,
  description,
  baselineMonthlySpending,
  baselineLoading,
  plannedMonthlySpending,
  forecastYears,
  setForecastYears,
  forecastAnnualReturn,
  setForecastAnnualReturn,
}: {
  title: string;
  description?: string;
  baselineMonthlySpending: number | null;
  baselineLoading: boolean;
  plannedMonthlySpending: number;
  forecastYears: number;
  setForecastYears: (years: number) => void;
  forecastAnnualReturn: number;
  setForecastAnnualReturn: (rate: number) => void;
}) {
  const monthlyDelta =
    baselineMonthlySpending === null ? null : baselineMonthlySpending - plannedMonthlySpending;

  const data = useMemo(() => {
    if (monthlyDelta === null) return [];
    return buildProjectedSavingsSeries(monthlyDelta, forecastYears, forecastAnnualReturn);
  }, [forecastAnnualReturn, forecastYears, monthlyDelta]);

  const finalProjected = data.length > 0 ? data[data.length - 1]!.projected : 0;

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base">{title}</CardTitle>
        {description && <CardDescription>{description}</CardDescription>}
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap items-end gap-4">
          <div>
            <div className="text-xs text-muted-foreground">Baseline (monthly)</div>
            <div className="text-lg font-semibold">
              {baselineLoading ? '—' : formatCurrency(baselineMonthlySpending ?? 0)}
            </div>
          </div>
          <div>
            <div className="text-xs text-muted-foreground">Planned (monthly)</div>
            <div className="text-lg font-semibold">{formatCurrency(plannedMonthlySpending)}</div>
          </div>
          <div>
            <div className="text-xs text-muted-foreground">Extra savings (monthly)</div>
            <div className={`text-lg font-semibold ${monthlyDelta !== null && monthlyDelta >= 0 ? 'text-green-600' : 'text-red-600'}`}>
              {monthlyDelta === null ? '—' : formatCurrency(monthlyDelta)}
            </div>
          </div>
          <div className="ml-auto flex items-center gap-3">
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground">Horizon</span>
              <Select
                value={String(forecastYears)}
                onChange={(e) => setForecastYears(Number(e.target.value))}
                className="h-9 w-28"
              >
                <option value="1">1 year</option>
                <option value="2">2 years</option>
                <option value="3">3 years</option>
                <option value="5">5 years</option>
                <option value="10">10 years</option>
              </Select>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground">Return</span>
              <Input
                type="number"
                inputMode="decimal"
                min="0"
                step="0.1"
                value={String(forecastAnnualReturn)}
                onChange={(e) => setForecastAnnualReturn(Number(e.target.value))}
                className="h-9 w-24"
              />
              <span className="text-xs text-muted-foreground">%</span>
            </div>
          </div>
        </div>

        {baselineLoading ? (
          <div className="flex h-[240px] items-center justify-center text-muted-foreground">
            Calculating baseline...
          </div>
        ) : baselineMonthlySpending === null ? (
          <div className="flex h-[240px] items-center justify-center text-muted-foreground">
            Not enough recent data to forecast yet.
          </div>
        ) : data.length === 0 ? (
          <div className="flex h-[240px] items-center justify-center text-muted-foreground">
            No forecast available.
          </div>
        ) : (
          <div className="space-y-2">
            <div className="text-sm text-muted-foreground">
              Projected cumulative extra savings: <span className="font-medium text-foreground">{formatCurrency(finalProjected)}</span>
            </div>
            <ResponsiveContainer width="100%" height={240}>
              <AreaChart data={data}>
                <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                <XAxis
                  dataKey="month"
                  tickFormatter={(value) => {
                    const m = Number(value);
                    return m % 12 === 0 ? `Y${m / 12}` : '';
                  }}
                  className="text-xs"
                />
                <YAxis
                  tickFormatter={(value) => formatCurrency(Number(value))}
                  className="text-xs"
                  width={80}
                />
                <Tooltip
                  formatter={(value: number) => formatCurrency(Number(value))}
                  labelFormatter={(label) => `Month ${label}`}
                  contentStyle={{
                    backgroundColor: 'var(--card)',
                    border: '1px solid var(--border)',
                    borderRadius: '3px',
                  }}
                />
                <Area
                  type="monotone"
                  dataKey="projected"
                  stroke="var(--accent)"
                  fill="var(--accent)"
                  fillOpacity={0.2}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function CompactSummaryCard({
  title,
  spending,
  savings,
}: {
  title: string;
  spending: { budgetedMonthly: number; actualMonthly: number };
  savings: { budgetedMonthly: number; actualMonthly: number };
}) {
  const spendingRemaining = spending.budgetedMonthly - spending.actualMonthly;
  const savingsDelta = savings.budgetedMonthly - savings.actualMonthly;

  return (
    <Card>
      <CardHeader className="pb-1.5 pt-3 px-3">
        <CardTitle className="text-xs font-semibold">{title}</CardTitle>
        <CardDescription className="text-[9px] leading-tight">Monthly</CardDescription>
      </CardHeader>
      <CardContent className="pt-0 px-3 pb-3 space-y-1.5">
        <div className="flex items-center gap-1">
          <div className="text-[9px] font-medium text-muted-foreground w-16">Spending</div>
          <div className="flex-1 grid grid-cols-3 gap-1">
            <div className="rounded-notion border border-border p-1">
              <div className="text-[8px] font-medium text-muted-foreground">Budget</div>
              <div className="text-[11px] font-semibold leading-tight">{formatCurrency(spending.budgetedMonthly)}</div>
            </div>
            <div className="rounded-notion border border-border p-1">
              <div className="text-[8px] font-medium text-muted-foreground">Actual</div>
              <div className="text-[11px] font-semibold leading-tight">{formatCurrency(spending.actualMonthly)}</div>
            </div>
            <div className="rounded-notion border border-border p-1">
              <div className="text-[8px] font-medium text-muted-foreground">Remaining</div>
              <div className={`text-[11px] font-semibold leading-tight ${spendingRemaining >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {formatCurrency(spendingRemaining)}
              </div>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-1 pt-1 border-t border-border">
          <div className="text-[9px] font-medium text-muted-foreground w-16">Savings</div>
          <div className="flex-1 grid grid-cols-3 gap-1">
            <div className="rounded-notion border border-border p-1">
              <div className="text-[8px] font-medium text-muted-foreground">Budget</div>
              <div className="text-[11px] font-semibold leading-tight">{formatCurrency(savings.budgetedMonthly)}</div>
            </div>
            <div className="rounded-notion border border-border p-1">
              <div className="text-[8px] font-medium text-muted-foreground">Actual</div>
              <div className="text-[11px] font-semibold leading-tight">{formatCurrency(savings.actualMonthly)}</div>
            </div>
            <div className="rounded-notion border border-border p-1">
              <div className="text-[8px] font-medium text-muted-foreground">Delta</div>
              <div className={`text-[11px] font-semibold leading-tight ${savingsDelta >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {formatCurrency(savingsDelta)}
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function CategoryBreakdownAreaChart({
  categories,
  budgetedByCategoryId,
  actualByCategoryId,
  actualYtdByCategoryId,
  mode,
  totalIncome,
  monthsElapsed,
}: {
  categories: any[];
  budgetedByCategoryId: Map<string, number>;
  actualByCategoryId: Map<string, number>;
  actualYtdByCategoryId: Map<string, number>;
  mode: 'budgeted' | 'actual';
  totalIncome: number;
  monthsElapsed: number;
}) {
  const monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  const data = useMemo(() => {
    const spendingCategories = categories.filter((c) => {
      if (String(c.id) === 'uncategorized') return false;
      const groupName = c.group_name ?? null;
      return !isExcludedFromSpending(groupName, c);
    });

    const chartData: Array<Record<string, any>> = [];
    for (let month = 1; month <= 12; month++) {
      const monthData: Record<string, any> = { month, monthName: monthNames[month - 1] };
      for (const cat of spendingCategories) {
        const categoryId = String(cat.id);
        let monthlyAmount = 0;
        if (mode === 'budgeted') {
          monthlyAmount = budgetedByCategoryId.get(categoryId) || 0;
        } else {
          // For actual mode, use average monthly spending (YTD / months elapsed)
          const ytd = actualYtdByCategoryId.get(categoryId) || 0;
          monthlyAmount = monthsElapsed > 0 ? ytd / monthsElapsed : 0;
        }
        monthData[cat.name] = monthlyAmount;
      }
      chartData.push(monthData);
    }
    return chartData;
  }, [categories, budgetedByCategoryId, actualByCategoryId, actualYtdByCategoryId, mode, monthsElapsed]);

  const spendingCategories = useMemo(() => {
    return categories.filter((c) => {
      if (String(c.id) === 'uncategorized') return false;
      const groupName = c.group_name ?? null;
      const categoryName = c.name ?? null;
      return !isExcludedFromSpending(groupName, categoryName);
    });
  }, [categories]);

  if (spendingCategories.length === 0) {
    return (
      <div className="flex h-[300px] items-center justify-center text-muted-foreground">
        No spending categories available.
      </div>
    );
  }

  return (
    <div className="w-full">
      <ResponsiveContainer width="100%" height={400}>
        <AreaChart data={data} margin={{ top: 10, right: 30, left: 0, bottom: 100 }}>
          <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
          <XAxis
            dataKey="monthName"
            className="text-xs"
            angle={-45}
            textAnchor="end"
            height={80}
          />
          <YAxis
            tickFormatter={(value) => formatCurrency(Number(value))}
            className="text-xs"
            width={80}
          />
          <Tooltip
            formatter={(value: number) => formatCurrency(Number(value))}
            labelFormatter={(label) => label}
            contentStyle={{
              backgroundColor: 'var(--card)',
              border: '1px solid var(--border)',
              borderRadius: '3px',
            }}
          />
          <Legend 
            wrapperStyle={{ paddingTop: '20px' }}
            iconType="square"
          />
          {spendingCategories.map((cat, idx) => {
            const categoryId = String(cat.id);
            const swatch =
              typeof cat.color === 'string' && cat.color.trim()
                ? cat.color
                : getCategoryColor(
                    {
                      color: cat.color ?? null,
                      parent_color: cat.parent_color ?? null,
                      group_name: cat.group_name ?? null,
                    },
                    idx,
                    spendingCategories.length || 1
                  );

            return (
              <Area
                key={categoryId}
                type="monotone"
                dataKey={cat.name}
                stackId="1"
                stroke={swatch}
                fill={swatch}
                fillOpacity={0.6}
              />
            );
          })}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

function BudgetForecastCharts({
  categories,
  budgetedByCategoryId,
  actualByCategoryId,
  actualYtdByCategoryId,
  totalIncome,
  monthsElapsed,
}: {
  categories: any[];
  budgetedByCategoryId: Map<string, number>;
  actualByCategoryId: Map<string, number>;
  actualYtdByCategoryId: Map<string, number>;
  totalIncome: number;
  monthsElapsed: number;
}) {
  const [mode, setMode] = useState<'budgeted' | 'actual'>('budgeted');

  const totalYearlySpending = useMemo(() => {
    let total = 0;
    for (const cat of categories) {
      const categoryId = String(cat.id);
      if (categoryId === 'uncategorized') continue;
      const groupName = cat.group_name ?? null;
      if (isExcludedFromSpending(groupName, cat)) continue;
      
      if (mode === 'budgeted') {
        const monthly = budgetedByCategoryId.get(categoryId) || 0;
        total += monthly * 12;
      } else {
        const ytd = actualYtdByCategoryId.get(categoryId) || 0;
        const avgMonthly = monthsElapsed > 0 ? ytd / monthsElapsed : 0;
        total += avgMonthly * 12;
      }
    }
    return total;
  }, [categories, budgetedByCategoryId, actualYtdByCategoryId, mode, monthsElapsed]);

  const remainingIncome = totalIncome - totalYearlySpending;
  const spendingPercentage = totalIncome > 0 ? (totalYearlySpending / totalIncome) * 100 : 0;

  return (
    <Card className="w-full">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <CardTitle className="text-base">Category Breakdown</CardTitle>
            <CardDescription className="text-xs">Monthly spending breakdown by category</CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant={mode === 'budgeted' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setMode('budgeted')}
              type="button"
            >
              Budgeted
            </Button>
            <Button
              variant={mode === 'actual' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setMode('actual')}
              type="button"
            >
              Actual
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        {totalIncome > 0 && (
          <div className="grid grid-cols-3 gap-3 rounded-notion border border-border p-3">
            <div>
              <div className="text-[10px] font-medium text-muted-foreground">Total Income</div>
              <div className="text-sm font-semibold">{formatCurrency(totalIncome)}</div>
            </div>
            <div>
              <div className="text-[10px] font-medium text-muted-foreground">
                {mode === 'budgeted' ? 'Budgeted' : 'Projected'} Spending
              </div>
              <div className="text-sm font-semibold">{formatCurrency(totalYearlySpending)}</div>
              <div className="text-[10px] text-muted-foreground">({spendingPercentage.toFixed(1)}% of income)</div>
            </div>
            <div>
              <div className="text-[10px] font-medium text-muted-foreground">Remaining</div>
              <div className={`text-sm font-semibold ${remainingIncome >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {formatCurrency(remainingIncome)}
              </div>
            </div>
          </div>
        )}

        {totalIncome > 0 && totalYearlySpending > totalIncome && (
          <div className="rounded-notion border border-red-500/50 bg-red-500/10 p-3">
            <div className="text-sm font-medium text-red-600">
              Warning: {mode === 'budgeted' ? 'Budgeted' : 'Projected'} spending exceeds income by {formatCurrency(Math.abs(remainingIncome))}
            </div>
          </div>
        )}

        <div className="w-full space-y-6">
          <div className="w-full">
            <h4 className="text-sm font-semibold mb-2">Monthly Breakdown (Area Chart)</h4>
            <CategoryBreakdownAreaChart
              categories={categories}
              budgetedByCategoryId={budgetedByCategoryId}
              actualByCategoryId={actualByCategoryId}
              actualYtdByCategoryId={actualYtdByCategoryId}
              mode={mode}
              totalIncome={totalIncome}
              monthsElapsed={monthsElapsed}
            />
          </div>

          <div className="w-full">
            <h4 className="text-sm font-semibold mb-2">Category Distribution (Pie Chart)</h4>
            <CategoryBreakdownPieChart
              categories={categories}
              budgetedByCategoryId={budgetedByCategoryId}
              actualYtdByCategoryId={actualYtdByCategoryId}
              mode={mode}
              totalIncome={totalIncome}
              monthsElapsed={monthsElapsed}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function CategoryBreakdownPieChart({
  categories,
  budgetedByCategoryId,
  actualYtdByCategoryId,
  mode,
  totalIncome,
  monthsElapsed,
}: {
  categories: any[];
  budgetedByCategoryId: Map<string, number>;
  actualYtdByCategoryId: Map<string, number>;
  mode: 'budgeted' | 'actual';
  totalIncome: number;
  monthsElapsed: number;
}) {
  const data = useMemo(() => {
    const spendingCategories = categories.filter((c) => {
      if (String(c.id) === 'uncategorized') return false;
      const groupName = c.group_name ?? null;
      return !isExcludedFromSpending(groupName, c);
    });

    const pieData: Array<{ name: string; value: number; categoryId: string; color: string }> = [];

    for (const cat of spendingCategories) {
      const categoryId = String(cat.id);
      let yearlyAmount = 0;

      if (mode === 'budgeted') {
        const monthly = budgetedByCategoryId.get(categoryId) || 0;
        yearlyAmount = monthly * 12;
      } else {
        // Project actual spending based on current YTD average
        const ytd = actualYtdByCategoryId.get(categoryId) || 0;
        const avgMonthly = monthsElapsed > 0 ? ytd / monthsElapsed : 0;
        yearlyAmount = avgMonthly * 12;
      }

      if (yearlyAmount > 0) {
        const swatch =
          typeof cat.color === 'string' && cat.color.trim()
            ? cat.color
            : getCategoryColor(
                {
                  color: cat.color ?? null,
                  parent_color: cat.parent_color ?? null,
                  group_name: cat.group_name ?? null,
                },
                spendingCategories.indexOf(cat),
                spendingCategories.length || 1
              );

        pieData.push({
          name: cat.name,
          value: yearlyAmount,
          categoryId,
          color: swatch,
        });
      }
    }

    return pieData.sort((a, b) => b.value - a.value);
  }, [categories, budgetedByCategoryId, actualYtdByCategoryId, mode, monthsElapsed]);

  if (data.length === 0) {
    return (
      <div className="flex h-[300px] items-center justify-center text-muted-foreground">
        No spending data available.
      </div>
    );
  }

  const totalSpending = data.reduce((sum, item) => sum + item.value, 0);
  const percentageOfIncome = totalIncome > 0 ? (totalSpending / totalIncome) * 100 : 0;

  return (
    <div className="space-y-4">
      <div className="text-sm text-muted-foreground text-center">
        {mode === 'budgeted' ? 'Budgeted' : 'Projected'} spending: <span className="font-medium text-foreground">{formatCurrency(totalSpending)}</span>
        {totalIncome > 0 && (
          <>
            {' '}
            ({percentageOfIncome.toFixed(1)}% of income)
          </>
        )}
      </div>
      <ResponsiveContainer width="100%" height={300}>
        <PieChart>
          <Pie
            data={data}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={({ name, percent }) => `${name}: ${(percent * 100).toFixed(0)}%`}
            outerRadius={100}
            fill="#8884d8"
            dataKey="value"
          >
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={entry.color} />
            ))}
          </Pie>
          <Tooltip
            formatter={(value: number) => formatCurrency(Number(value))}
            contentStyle={{
              backgroundColor: 'var(--card)',
              border: '1px solid var(--border)',
              borderRadius: '3px',
            }}
          />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
}

function FragmentGroup({
  groupName,
  categories,
  draftByCategoryId,
  actualMonthByCategoryId,
  actualYtdByCategoryId,
  monthsElapsed,
  expanded,
  onToggle,
  onChangeBudget,
}: {
  groupName: string;
  categories: any[];
  draftByCategoryId: Record<string, string>;
  actualMonthByCategoryId: Map<string, number>;
  actualYtdByCategoryId: Map<string, number>;
  monthsElapsed: number;
  expanded: boolean;
  onToggle: () => void;
  onChangeBudget: (categoryId: string, value: string) => void;
}) {
  const groupTotals = categories.reduce(
    (acc, c: any) => {
      const categoryId = String(c.id);
      const budgetRaw = draftByCategoryId[categoryId] ?? '';
      const budgetMonthly = budgetRaw === '' ? 0 : Number(budgetRaw);
      const budgetMonthlySafe = Number.isFinite(budgetMonthly) ? Math.max(0, budgetMonthly) : 0;
      const budgetYearly = budgetMonthlySafe * 12;

      const actualMonth = actualMonthByCategoryId.get(categoryId) || 0;
      const actualYtd = actualYtdByCategoryId.get(categoryId) || 0;

      acc.budgetMonthly += budgetMonthlySafe;
      acc.budgetYearly += budgetYearly;
      acc.actualMonth += actualMonth;
      acc.actualYtd += actualYtd;
      return acc;
    },
    { budgetMonthly: 0, budgetYearly: 0, actualMonth: 0, actualYtd: 0 }
  );

  const groupActualAvgMonthly = monthsElapsed > 0 ? groupTotals.actualYtd / monthsElapsed : 0;
  const groupRemaining = groupTotals.budgetMonthly - groupTotals.actualMonth;
  const groupBaseColor =
    (categories.find((c: any) => c?.parent_color)?.parent_color as string | undefined) ||
    (categories.find((c: any) => c?.color)?.color as string | undefined) ||
    '#95A5A6';

  return (
    <>
      <tr className="bg-muted/30 border-t border-border">
        <td
          className="p-2 text-xs font-semibold whitespace-nowrap sticky left-0 z-20 border-r border-border"
          style={{
            borderLeft: `4px solid ${groupBaseColor}`,
            backgroundColor: `color-mix(in srgb, ${groupBaseColor} 10%, var(--card))`,
            minWidth: '200px',
            width: '200px',
          }}
        >
          <button
            type="button"
            onClick={onToggle}
            className="inline-flex items-center gap-1 text-left hover:underline"
          >
            {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
            <span>{groupName}</span>
            <span className="text-[10px] text-muted-foreground">({categories.length})</span>
          </button>
        </td>
        <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">{formatCurrency(groupTotals.budgetMonthly)}</td>
        <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">{formatCurrency(groupTotals.budgetYearly)}</td>
        <td className={`p-2 text-xs font-semibold whitespace-nowrap ${groupRemaining >= 0 ? 'text-green-600' : 'text-red-600'}`}>
          {formatCurrency(groupRemaining)}
        </td>
        <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">{formatCurrency(groupTotals.actualMonth)}</td>
        <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">{formatCurrency(groupActualAvgMonthly)}</td>
        <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">{formatCurrency(groupTotals.actualYtd)}</td>
      </tr>

      {expanded &&
        categories.map((c: any, idx: number) => {
          const categoryId = String(c.id);
          const isUncategorized = categoryId === 'uncategorized';
          const budgetRaw = draftByCategoryId[categoryId] ?? '';
          const budgetMonthly = budgetRaw === '' ? 0 : Number(budgetRaw);
          const budgetMonthlySafe = Number.isFinite(budgetMonthly) ? Math.max(0, budgetMonthly) : 0;
          const budgetYearly = budgetMonthlySafe * 12;

          const actualMonth = actualMonthByCategoryId.get(categoryId) || 0;
          const actualYtd = actualYtdByCategoryId.get(categoryId) || 0;
          const actualAvgMonthly = monthsElapsed > 0 ? actualYtd / monthsElapsed : 0;
          const remaining = budgetMonthlySafe - actualMonth;

          const swatch =
            typeof c.color === 'string' && c.color.trim()
              ? c.color
              : getCategoryColor(
                  {
                    color: c.color ?? null,
                    parent_color: c.parent_color ?? null,
                    group_name: c.group_name ?? null,
                  },
                  idx,
                  categories.length || 1
                );

          return (
            <tr key={categoryId} className="group border-t border-border hover:bg-hover">
              <td
                className="p-2 text-xs font-medium whitespace-nowrap sticky left-0 z-10 border-r border-border"
                style={{ 
                  backgroundColor: `color-mix(in srgb, ${swatch} 12%, var(--card))`,
                  minWidth: '200px',
                  width: '200px',
                }}
              >
                <div className="flex items-center gap-2">
                  <span
                    className="h-2.5 w-2.5 rounded-full border border-border/60"
                    style={{ backgroundColor: swatch }}
                  />
                  <span className="truncate">{c.name}</span>
                </div>
              </td>
              <td className="p-2 text-xs whitespace-nowrap">
                <Input
                  type="number"
                  inputMode="decimal"
                  step="0.01"
                  min="0"
                  value={budgetRaw}
                  onChange={(e) => onChangeBudget(categoryId, e.target.value)}
                  disabled={isUncategorized}
                  placeholder={isUncategorized ? '—' : '0'}
                  className="h-7 w-24 px-2 text-xs"
                />
              </td>
              <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">
                {isUncategorized ? '—' : formatCurrency(budgetYearly)}
              </td>
              <td className={`p-2 text-xs font-semibold whitespace-nowrap ${remaining >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {isUncategorized ? '—' : formatCurrency(remaining)}
              </td>
              <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">{formatCurrency(actualMonth)}</td>
              <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">{formatCurrency(actualAvgMonthly)}</td>
              <td className="p-2 text-xs text-muted-foreground whitespace-nowrap">{formatCurrency(actualYtd)}</td>
            </tr>
          );
        })}
    </>
  );
}


