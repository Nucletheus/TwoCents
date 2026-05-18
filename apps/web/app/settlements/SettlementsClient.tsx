'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import { createClient } from '@/lib/supabase/client';
import {
  calculateSharedSettlements,
  formatCurrency,
  buildEvenSplit,
  getMonthRange,
  getYearRange,
  normalizeCategoryGroupColors,
  generateColorVariations,
  getCategoryColor,
} from '@twocents/shared';
import MainLayout from '../components/MainLayout';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Input from '../components/ui/Input';
import Button from '../components/ui/Button';
import { Wallet, ChevronDown } from 'lucide-react';
import { useHousehold } from '../components/HouseholdProvider';
import SharedCategoriesDialog from './SharedCategoriesDialog';

export default function SettlementsClient() {
  const supabase = createClient();
  const { selectedHouseholdId } = useHousehold();
  const [members, setMembers] = useState<any[]>([]);
  const [categories, setCategories] = useState<any[]>([]);
  const [selectedCategoryIds, setSelectedCategoryIds] = useState<string[]>([]);
  const [savedCategoryIds, setSavedCategoryIds] = useState<string[]>([]);
  const [sharedSplits, setSharedSplits] = useState<Record<string, Record<string, number>>>({});
  const [settlements, setSettlements] = useState<any[]>([]);
  const [balances, setBalances] = useState<Record<string, number>>({});
  const [totalSharedSpend, setTotalSharedSpend] = useState(0);
  const [categoryTotals, setCategoryTotals] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(false);
  const [hasCalculated, setHasCalculated] = useState(false);
  const [savingSharedConfig, setSavingSharedConfig] = useState(false);
  const [timeframe, setTimeframe] = useState<'month' | 'year' | 'all' | 'custom'>('month');
  const [selectedMonthKeys, setSelectedMonthKeys] = useState<string[]>(() => {
    const now = new Date();
    return [`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`];
  });
  const [customDateFrom, setCustomDateFrom] = useState('');
  const [customDateTo, setCustomDateTo] = useState('');
  const [calculateError, setCalculateError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  const [sharedCategoriesOpen, setSharedCategoriesOpen] = useState(false);
  const [draftCategoryIds, setDraftCategoryIds] = useState<string[]>([]);
  const [draftSplits, setDraftSplits] = useState<Record<string, Record<string, number>>>({});
  const [expenseDetails, setExpenseDetails] = useState<any[]>([]);
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set());

  const calcDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const calcRunIdRef = useRef(0);

  useEffect(() => {
    if (!selectedHouseholdId) {
      setMembers([]);
      setCategories([]);
      setSelectedCategoryIds([]);
      setSavedCategoryIds([]);
      setSharedSplits({});
      setDraftCategoryIds([]);
      setDraftSplits({});
      setSharedCategoriesOpen(false);
      setCalculateError(null);
      setSaveError(null);
      setLoading(false);
      setHasCalculated(false);
      return;
    }

    fetchHouseholdMembers();
    fetchCategories();
    fetchSharedCategorySplits();
  }, [selectedHouseholdId]);

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

  const memberIds = useMemo(() => members.map((m) => m.user_id), [members]);

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
        if (set.size === 1) return current; // keep at least one month selected
        set.delete(monthKey);
      } else {
        set.add(monthKey);
      }
      // stable ordering for deps; newest first based on YYYY-MM lexical sort
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

  const openSharedCategoriesDialog = () => {
    setDraftCategoryIds(selectedCategoryIds);
    setDraftSplits(sharedSplits);
    setSaveError(null);
    setSharedCategoriesOpen(true);
  };

  const closeSharedCategoriesDialog = () => {
    setSharedCategoriesOpen(false);
  };

  const memberNameMap = useMemo(() => {
    const map = new Map<string, string>();
    members.forEach((m: any) => {
      const profile = Array.isArray(m.profiles) ? m.profiles[0] : m.profiles;
      map.set(m.user_id, profile?.name || profile?.email || 'Unknown');
    });
    return map;
  }, [members]);

  const normalizeSplitForMembers = (split?: Record<string, number>) => {
    if (memberIds.length === 0) return {};
    if (memberIds.length === 1) {
      return { [memberIds[0]]: 100 };
    }

    const even = buildEvenSplit(memberIds);
    const combined: Record<string, number> = {};
    memberIds.forEach((id) => {
      combined[id] = split?.[id] ?? even[id] ?? 0;
    });

    const total = memberIds.reduce((sum, id) => sum + (combined[id] || 0), 0);
    if (total === 0) {
      return even;
    }

    const factor = 100 / total;
    const normalized: Record<string, number> = {};
    let running = 0;

    memberIds.forEach((id, index) => {
      if (index === memberIds.length - 1) {
        normalized[id] = parseFloat((100 - running).toFixed(2));
      } else {
        const value = parseFloat(((combined[id] || 0) * factor).toFixed(2));
        normalized[id] = value;
        running += value;
      }
    });

    return normalized;
  };

  useEffect(() => {
    if (memberIds.length === 0) return;
    setSharedSplits((prev) => {
      const next: Record<string, Record<string, number>> = {};
      selectedCategoryIds.forEach((catId) => {
        next[catId] = normalizeSplitForMembers(prev[catId]);
      });
      return next;
    });
  }, [memberIds.join(','), selectedCategoryIds.join(',')]);

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
    } catch (err) {
      console.error('Error fetching categories:', err);
    }
  };

  const fetchHouseholdMembers = async () => {
    if (!selectedHouseholdId) {
      setMembers([]);
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
          setMembers([]);
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

        setMembers(
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

      setMembers(normalized);
    } catch (err) {
      console.error('Error fetching household members:', err);
      setMembers([]);
    }
  };

  const fetchSharedCategorySplits = async () => {
    if (!selectedHouseholdId) {
      setSharedSplits({});
      setSelectedCategoryIds([]);
      setSavedCategoryIds([]);
      return;
    }

    try {
      const { data, error } = await supabase
        .from('shared_category_splits')
        .select('category_id, user_id, percentage')
        .eq('household_id', selectedHouseholdId);

      if (error) throw error;

      const mapped: Record<string, Record<string, number>> = {};
      (data || []).forEach((row: any) => {
        if (!mapped[row.category_id]) mapped[row.category_id] = {};
        mapped[row.category_id][row.user_id] = Number(row.percentage) || 0;
      });

      const normalized: Record<string, Record<string, number>> = {};
      Object.entries(mapped).forEach(([categoryId, split]) => {
        normalized[categoryId] = normalizeSplitForMembers(split);
      });

      setSharedSplits(normalized);
      const categoryIds = Object.keys(normalized);
      setSelectedCategoryIds(categoryIds);
      setSavedCategoryIds(categoryIds);
    } catch (err) {
      console.error('Error fetching shared category splits:', err);
    }
  };

  const toggleDraftCategory = (categoryId: string) => {
    setDraftCategoryIds((prev) => {
      const exists = prev.includes(categoryId);
      setDraftSplits((current) => {
        const next = { ...current };
        if (exists) {
          delete next[categoryId];
        } else {
          next[categoryId] = normalizeSplitForMembers(current[categoryId]);
        }
        return next;
      });
      return exists ? prev.filter((id) => id !== categoryId) : [...prev, categoryId];
    });
  };

  const setDraftCategoriesShared = (categoryIds: string[], shared: boolean) => {
    if (categoryIds.length === 0) return;

    setDraftCategoryIds((prev) => {
      const next = new Set(prev);
      if (shared) {
        categoryIds.forEach((id) => next.add(id));
      } else {
        categoryIds.forEach((id) => next.delete(id));
      }
      return Array.from(next);
    });

    setDraftSplits((prev) => {
      const next = { ...prev };
      if (shared) {
        categoryIds.forEach((id) => {
          next[id] = normalizeSplitForMembers(prev[id]);
        });
      } else {
        categoryIds.forEach((id) => {
          delete next[id];
        });
      }
      return next;
    });
  };

  const handleDraftSplitChange = (categoryId: string, memberId: string, value: string) => {
    const parsed = Math.min(100, Math.max(0, Number(value) || 0));
    setDraftSplits((prev) => {
      const current = prev[categoryId] || {};

      // Couple-friendly behavior: for 2 members, keep the other member as the remainder.
      if (memberIds.length === 2) {
        const otherId = memberIds.find((id) => id !== memberId);
        if (!otherId) {
          return { ...prev, [categoryId]: normalizeSplitForMembers({ ...current, [memberId]: parsed }) };
        }
        const primary = parseFloat(parsed.toFixed(2));
        const remainder = parseFloat((100 - primary).toFixed(2));
        return {
          ...prev,
          [categoryId]: {
            ...current,
            [memberId]: primary,
            [otherId]: remainder,
          },
        };
      }

      const updated = { ...current, [memberId]: parsed };
      const normalized = normalizeSplitForMembers(updated);
      return { ...prev, [categoryId]: normalized };
    });
  };

  const handleSaveDraftSharedConfig = async () => {
    if (!selectedHouseholdId) return;
    setSavingSharedConfig(true);
    setSaveError(null);
    try {
      const normalizedSplits: Record<string, Record<string, number>> = {};
      draftCategoryIds.forEach((categoryId) => {
        normalizedSplits[categoryId] = normalizeSplitForMembers(draftSplits[categoryId]);
      });

      const rows: any[] = [];
      draftCategoryIds.forEach((categoryId) => {
        const split = normalizedSplits[categoryId] || normalizeSplitForMembers();
        memberIds.forEach((memberId) => {
          rows.push({
            household_id: selectedHouseholdId,
            category_id: categoryId,
            user_id: memberId,
            percentage: split[memberId] ?? 0,
          });
        });
      });

      if (rows.length > 0) {
        const { error: upsertError } = await supabase
          .from('shared_category_splits')
          .upsert(rows, { onConflict: 'household_id,category_id,user_id' });
        if (upsertError) throw upsertError;
      }

      const toDelete = savedCategoryIds.filter((id) => !draftCategoryIds.includes(id));
      if (toDelete.length > 0) {
        const { error: deleteError } = await supabase
          .from('shared_category_splits')
          .delete()
          .eq('household_id', selectedHouseholdId)
          .in('category_id', toDelete);
        if (deleteError) throw deleteError;
      }

      setSelectedCategoryIds(draftCategoryIds);
      setSharedSplits(normalizedSplits);
      setSavedCategoryIds(draftCategoryIds);
      setSharedCategoriesOpen(false);
    } catch (err) {
      console.error('Error saving shared splits:', err);
      setSaveError('Could not save shared splits. Please try again.');
    } finally {
      setSavingSharedConfig(false);
    }
  };

  const resolveDateRange = (): { from?: string; to?: string; ranges?: Array<{ from: string; to: string }> } => {
    if (timeframe === 'month') {
      const keys = selectedMonthKeys.length > 0 ? selectedMonthKeys : [];
      const ranges = keys
        .map((key) => {
          const range = getMonthRange(coerceMonthKeyToDate(key));
          return {
            from: range.start.toISOString().slice(0, 10),
            to: range.end.toISOString().slice(0, 10),
          };
        })
        .sort((a, b) => a.from.localeCompare(b.from));

      if (ranges.length <= 1) {
        return { from: ranges[0]?.from, to: ranges[0]?.to };
      }
      return { ranges };
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

  const fetchExpenseDetails = async () => {
    if (!selectedHouseholdId || selectedCategoryIds.length === 0) {
      setExpenseDetails([]);
      return;
    }

    try {
      const range = resolveDateRange();
      let query = supabase
        .from('expenses')
        .select(
          `
          id,
          amount,
          payer_id,
          category_id,
          date,
          description,
          vendor,
          categories (
            id,
            name,
            icon,
            color,
            group_name,
            parent_color
          )
        `
        )
        .eq('household_id', selectedHouseholdId)
        .in('category_id', selectedCategoryIds);

      const ranges = (range.ranges || []).filter((r) => r.from || r.to);
      if (ranges.length > 0) {
        const parts = ranges.map((r) => {
          const from = r.from;
          const to = r.to;
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
        if (range.from) {
          query = query.gte('date', range.from);
        }
        if (range.to) {
          query = query.lte('date', range.to);
        }
      }

      const { data: expenses, error } = await query.order('date', { ascending: false });

      if (error) throw error;

      // Normalize category data (handle array vs single object)
      const normalizedExpenses = (expenses || []).map((expense: any) => {
        const category = Array.isArray(expense.categories) ? expense.categories[0] : expense.categories;
        return {
          ...expense,
          category: category || null,
          amount: Math.abs(Number(expense.amount) || 0), // Use absolute value for display
        };
      });

      setExpenseDetails(normalizedExpenses);
    } catch (err) {
      console.error('Error fetching expense details:', err);
      setExpenseDetails([]);
    }
  };

  useEffect(() => {
    const ready =
      selectedHouseholdId &&
      selectedCategoryIds.length > 0 &&
      memberIds.length > 0 &&
      (timeframe !== 'month' || selectedMonthKeys.length > 0) &&
      (timeframe !== 'custom' || (customDateFrom && customDateTo));

    if (!ready) {
      if (calcDebounceRef.current) {
        clearTimeout(calcDebounceRef.current);
        calcDebounceRef.current = null;
      }
      setSettlements([]);
      setBalances({});
      setTotalSharedSpend(0);
      setCategoryTotals({});
      setExpenseDetails([]);
      setLoading(false);
      setHasCalculated(false);
      return;
    }

    // Debounce recalculation so users can multi-select months quickly without UI flicker.
    if (calcDebounceRef.current) {
      clearTimeout(calcDebounceRef.current);
      calcDebounceRef.current = null;
    }

    const runId = ++calcRunIdRef.current;
    calcDebounceRef.current = setTimeout(() => {
      const calculate = async () => {
        setLoading(true);
        setCalculateError(null);
        try {
          const range = resolveDateRange();
          const {
            settlements: rawSettlements,
            balances: rawBalances,
            totalSharedSpend: rawTotalSharedSpend,
            categoryTotals: rawCategoryTotals,
          } = await calculateSharedSettlements({
            supabase,
            householdId: selectedHouseholdId!,
            categoryIds: selectedCategoryIds,
            memberIds,
            splitsByCategory: sharedSplits,
            dateFrom: range.from,
            dateTo: range.to,
            dateRanges: range.ranges,
          });

          if (calcRunIdRef.current !== runId) return;

          const settlementsWithNames = rawSettlements.map((s: any) => ({
            ...s,
            fromName: memberNameMap.get(s.from) || 'Unknown',
            toName: memberNameMap.get(s.to) || 'Unknown',
          }));

          setSettlements(settlementsWithNames);
          setBalances(rawBalances);
          setTotalSharedSpend(rawTotalSharedSpend || 0);
          setCategoryTotals(rawCategoryTotals || {});
          setHasCalculated(true);

          // Fetch expense details after calculation completes
          await fetchExpenseDetails();
        } catch (err) {
          if (calcRunIdRef.current !== runId) return;
          console.error('Error calculating settlements:', err);
          setCalculateError('Unable to calculate settlements.');
          setTotalSharedSpend(0);
          setCategoryTotals({});
          setHasCalculated(false);
        } finally {
          if (calcRunIdRef.current === runId) {
            setLoading(false);
          }
        }
      };

      calculate();
    }, 250);

    return () => {
      if (calcDebounceRef.current) {
        clearTimeout(calcDebounceRef.current);
        calcDebounceRef.current = null;
      }
    };
  }, [
    selectedHouseholdId,
    selectedCategoryIds.join(','),
    JSON.stringify(sharedSplits),
    memberIds.join(','),
    timeframe,
    selectedMonthKeys.join(','),
    customDateFrom,
    customDateTo,
    memberNameMap,
  ]);

  const selectedCategories = selectedCategoryIds
    .map((id) => categories.find((c: any) => c.id === id))
    .filter(Boolean) as any[];

  const groupedExpenses = useMemo(() => {
    if (expenseDetails.length === 0) return {};

    const groups: Record<
      string,
      {
        groupName: string;
        expenses: any[];
        total: number;
        groupColor: string | null;
      }
    > = {};

    expenseDetails.forEach((expense: any) => {
      const groupName = expense.category?.group_name || 'Other';
      if (!groups[groupName]) {
        const groupCategories = categories.filter((c: any) => (c.group_name || 'Other') === groupName);
        const groupColor =
          groupCategories.find((c: any) => c.parent_color)?.parent_color ||
          groupCategories.find((c: any) => c.color)?.color ||
          null;

        groups[groupName] = {
          groupName,
          expenses: [],
          total: 0,
          groupColor,
        };
      }

      groups[groupName].expenses.push(expense);
      groups[groupName].total += expense.amount;
    });

    // Sort expenses within each group by date (newest first)
    Object.values(groups).forEach((group) => {
      group.expenses.sort((a, b) => {
        const dateA = new Date(a.date).getTime();
        const dateB = new Date(b.date).getTime();
        return dateB - dateA;
      });
    });

    return groups;
  }, [expenseDetails, categories]);

  const groupedSelectedCategories = useMemo(() => {
    const groups: Record<string, any[]> = {};
    const ungrouped: any[] = [];

    selectedCategories.forEach((cat: any) => {
      if (cat.group_name) {
        if (!groups[cat.group_name]) groups[cat.group_name] = [];
        groups[cat.group_name].push(cat);
      } else {
        ungrouped.push(cat);
      }
    });

    // Sort categories within each group
    Object.keys(groups).forEach((groupName) => {
      groups[groupName].sort((a, b) => a.name.localeCompare(b.name));
    });
    ungrouped.sort((a, b) => a.name.localeCompare(b.name));

    // Sort group names
    const sortedGroupNames = Object.keys(groups).sort((a, b) => a.localeCompare(b));

    return { groups, sortedGroupNames, ungrouped };
  }, [selectedCategories]);

  return (
    <MainLayout
      pageHeader={{
        title: 'Settlements',
        subtitle: 'See who owes whom',
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
      <div className="space-y-6">
        <SharedCategoriesDialog
          isOpen={sharedCategoriesOpen}
          onClose={closeSharedCategoriesDialog}
          categories={categories as any}
          memberIds={memberIds}
          memberNameMap={memberNameMap}
          selectedCategoryIds={draftCategoryIds}
          splitsByCategory={draftSplits}
          onToggleCategory={toggleDraftCategory}
          onSetCategoriesShared={setDraftCategoriesShared}
          onSplitChange={handleDraftSplitChange}
          onSave={handleSaveDraftSharedConfig}
          saving={savingSharedConfig}
          error={saveError}
        />

        <Card>
          <CardHeader className="p-4 pb-3">
            <div className="flex items-start justify-between gap-3">
              <div>
                <CardTitle>Shared categories</CardTitle>
                <CardDescription>Pick what’s shared and how it’s split</CardDescription>
              </div>
              <Button type="button" variant="secondary" size="sm" onClick={openSharedCategoriesDialog}>
                Configure
              </Button>
            </div>
          </CardHeader>
          <CardContent className="p-4 pt-0">
            {selectedCategories.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No shared categories selected yet. Configure shared categories to calculate a split.
              </p>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {groupedSelectedCategories.sortedGroupNames.map((groupName) => {
                  const groupCats = groupedSelectedCategories.groups[groupName];
                  const groupBaseColor =
                    groupCats.find((c: any) => c.parent_color)?.parent_color ||
                    groupCats.find((c: any) => c.color)?.color ||
                    null;

                  return (
                    <div key={groupName} className="space-y-1.5">
                      <div className="flex items-center gap-1.5">
                        {groupBaseColor && (
                          <span
                            className="h-3 w-0.5 rounded-full"
                            style={{ backgroundColor: groupBaseColor }}
                          />
                        )}
                        <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                          {groupName}
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-1.5 pl-2">
                        {groupCats.map((cat: any, idx: number) => {
                          const catColor = groupBaseColor
                            ? generateColorVariations(groupBaseColor, idx, groupCats.length)
                            : getCategoryColor(cat, idx, groupCats.length);

                          // Determine text color based on background brightness
                          const hex = catColor.replace('#', '').padEnd(6, '0');
                          const r = parseInt(hex.substring(0, 2), 16) || 0;
                          const g = parseInt(hex.substring(2, 4), 16) || 0;
                          const b = parseInt(hex.substring(4, 6), 16) || 0;
                          const brightness = (r * 299 + g * 587 + b * 114) / 1000;
                          const textColor = brightness > 128 ? '#000000' : '#ffffff';

                          return (
                            <span
                              key={cat.id}
                              className="inline-flex items-center rounded-notion border border-border/50 px-1.5 py-0.5 text-[11px] font-medium"
                              style={{
                                backgroundColor: catColor,
                                color: textColor,
                              }}
                            >
                              {cat.icon && <span className="mr-1 text-[10px]">{cat.icon}</span>}
                              {cat.name}
                            </span>
                          );
                        })}
                      </div>
                    </div>
                  );
                })}
                {groupedSelectedCategories.ungrouped.length > 0 && (
                  <div className="space-y-1.5">
                    <div className="flex items-center gap-1.5">
                      <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                        Other
                      </span>
                    </div>
                    <div className="flex flex-wrap gap-1.5 pl-2">
                      {groupedSelectedCategories.ungrouped.map((cat: any, idx: number) => {
                        const catColor = getCategoryColor(cat, idx, groupedSelectedCategories.ungrouped.length);

                        // Determine text color based on background brightness
                        const hex = catColor.replace('#', '').padEnd(6, '0');
                        const r = parseInt(hex.substring(0, 2), 16) || 0;
                        const g = parseInt(hex.substring(2, 4), 16) || 0;
                        const b = parseInt(hex.substring(4, 6), 16) || 0;
                        const brightness = (r * 299 + g * 587 + b * 114) / 1000;
                        const textColor = brightness > 128 ? '#000000' : '#ffffff';

                        return (
                          <span
                            key={cat.id}
                            className="inline-flex items-center rounded-notion border border-border/50 px-1.5 py-0.5 text-[11px] font-medium"
                            style={{
                              backgroundColor: catColor,
                              color: textColor,
                            }}
                          >
                            {cat.icon && <span className="mr-1 text-[10px]">{cat.icon}</span>}
                            {cat.name}
                          </span>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            )}
          </CardContent>
        </Card>

        {calculateError && (
          <Card>
            <CardContent className="py-3 text-sm text-red-600">{calculateError}</CardContent>
          </Card>
        )}

        {selectedCategoryIds.length === 0 ? (
          <Card>
            <CardContent className="py-8 text-center">
              <Wallet className="mx-auto mb-4 h-12 w-12 text-muted-foreground" />
              <p className="text-muted-foreground">Choose shared categories to calculate a settle-up amount.</p>
              <div className="mt-4 flex justify-center">
                <Button type="button" variant="secondary" onClick={openSharedCategoriesDialog}>
                  Configure shared categories
                </Button>
              </div>
            </CardContent>
          </Card>
        ) : memberIds.length <= 1 ? (
          <Card>
            <CardContent className="py-8 text-center">
              <Wallet className="mx-auto mb-4 h-12 w-12 text-muted-foreground" />
              <p className="text-muted-foreground">Add another household member to split shared categories.</p>
            </CardContent>
          </Card>
        ) : timeframe === 'custom' && (!customDateFrom || !customDateTo) ? (
          <Card>
            <CardContent className="py-8 text-center">
              <p className="text-muted-foreground">Pick a From and To date to calculate settlements.</p>
            </CardContent>
          </Card>
        ) : loading && !hasCalculated ? (
          <Card>
            <CardContent className="py-8 text-center">
              <p className="text-muted-foreground">Calculating settlements...</p>
            </CardContent>
          </Card>
        ) : (
          <>
            <div className="grid gap-4 lg:grid-cols-3">
              <Card className="lg:col-span-2">
                <CardHeader>
                  <CardTitle>Settle up</CardTitle>
                  <CardDescription>
                    Total shared spend: {formatCurrency(totalSharedSpend)}
                    {loading && hasCalculated && <span className="ml-2 text-xs text-muted-foreground">Updating…</span>}
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {settlements.length === 0 ? (
                    <div className="rounded-notion border border-border p-4 text-center">
                      <p className="text-sm text-muted-foreground">No settlements needed. Everyone is balanced!</p>
                    </div>
                  ) : (
                    <>
                      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                        <div className="text-sm">
                          <span className="font-medium">{settlements[0].fromName}</span>
                          <span className="mx-2 text-muted-foreground">pays</span>
                          <span className="font-medium">{settlements[0].toName}</span>
                        </div>
                        <div className="text-2xl font-bold text-accent tabular-nums">
                          {formatCurrency(settlements[0].amount)}
                        </div>
                      </div>

                      {settlements.length > 1 && (
                        <div className="space-y-2">
                          <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                            Other transfers
                          </p>
                          <div className="space-y-2">
                            {settlements.slice(1, 4).map((s: any, idx: number) => (
                              <div
                                key={`${s.from}-${s.to}-${idx}`}
                                className="flex items-center justify-between rounded-notion border border-border px-3 py-2 text-sm"
                              >
                                <div className="min-w-0">
                                  <span className="font-medium">{s.fromName}</span>
                                  <span className="mx-2 text-muted-foreground">pays</span>
                                  <span className="font-medium">{s.toName}</span>
                                </div>
                                <span className="font-semibold text-accent tabular-nums">{formatCurrency(s.amount)}</span>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                    </>
                  )}
                </CardContent>
              </Card>

              <Card className="lg:col-span-1">
                <CardHeader>
                  <CardTitle>Net balances</CardTitle>
                  <CardDescription>Who is owed vs who owes</CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  {memberIds
                    .map((id) => ({
                      id,
                      name: memberNameMap.get(id) || 'Unknown',
                      balance: balances[id] ?? 0,
                    }))
                    .sort((a, b) => b.balance - a.balance)
                    .map((row) => {
                      const isOwed = row.balance > 0.01;
                      const owes = row.balance < -0.01;
                      const label = isOwed ? 'is owed' : owes ? 'owes' : 'is even';
                      const amount = formatCurrency(Math.abs(row.balance));
                      const color = isOwed ? 'text-accent' : owes ? 'text-red-600' : 'text-muted-foreground';
                      return (
                        <div key={row.id} className="flex items-center justify-between rounded-notion border border-border p-3">
                          <div className="min-w-0">
                            <p className="truncate text-sm font-medium">{row.name}</p>
                            <p className="text-xs text-muted-foreground">{label}</p>
                          </div>
                          <p className={`text-lg font-bold tabular-nums ${color}`}>{amount}</p>
                        </div>
                      );
                    })}
                </CardContent>
              </Card>
            </div>

            <Card>
              <CardHeader>
                <CardTitle>Category breakdown</CardTitle>
                <CardDescription>Transactions grouped by category for this timeframe</CardDescription>
              </CardHeader>
              <CardContent>
                {totalSharedSpend <= 0 || Object.keys(groupedExpenses).length === 0 ? (
                  <p className="text-sm text-muted-foreground">No shared spending found for this timeframe.</p>
                ) : (
                  <div className="space-y-2">
                    {Object.entries(groupedExpenses)
                      .sort(([a], [b]) => a.localeCompare(b))
                      .map(([groupName, groupData]) => {
                        const isExpanded = expandedGroups.has(groupName);
                        const groupHeaderStyle = groupData.groupColor
                          ? {
                              backgroundColor: groupData.groupColor + '15',
                            }
                          : undefined;

                        return (
                          <div key={groupName} className="rounded-notion border border-border overflow-hidden">
                            <button
                              type="button"
                              onClick={() => {
                                setExpandedGroups((prev) => {
                                  const next = new Set(prev);
                                  if (next.has(groupName)) {
                                    next.delete(groupName);
                                  } else {
                                    next.add(groupName);
                                  }
                                  return next;
                                });
                              }}
                              className="w-full px-3 py-2 flex items-center justify-between hover:bg-hover transition-colors text-left"
                              style={groupHeaderStyle}
                            >
                              <div className="flex items-center gap-2 min-w-0 flex-1">
                                {groupData.groupColor && (
                                  <span
                                    className="h-4 w-0.5 rounded-full shrink-0"
                                    style={{ backgroundColor: groupData.groupColor }}
                                  />
                                )}
                                <span className="text-sm font-medium truncate">{groupName}</span>
                                <span className="text-xs text-muted-foreground tabular-nums">
                                  ({groupData.expenses.length} {groupData.expenses.length === 1 ? 'transaction' : 'transactions'})
                                </span>
                              </div>
                              <div className="flex items-center gap-2 shrink-0">
                                <span className="text-sm font-semibold tabular-nums">{formatCurrency(groupData.total)}</span>
                                <ChevronDown
                                  className={`h-4 w-4 text-muted-foreground transition-transform ${isExpanded ? '' : '-rotate-90'}`}
                                />
                              </div>
                            </button>

                            {isExpanded && (
                              <div className="border-t border-border">
                                <div className="divide-y divide-border/30">
                                  {groupData.expenses.map((expense: any) => {
                                    const split = sharedSplits[expense.category_id] || buildEvenSplit(memberIds);
                                    const expenseDate = new Date(expense.date);
                                    const formattedDate = expenseDate.toLocaleDateString('en-US', {
                                      month: 'short',
                                      day: 'numeric',
                                      year: 'numeric',
                                    });
                                    const payerName = memberNameMap.get(expense.payer_id) || 'Unknown';
                                    const category = expense.category;
                                    const categoryColor = category?.parent_color || category?.color || null;

                                    // Determine text color based on background brightness
                                    let textColor = '#000000';
                                    let borderColor = 'rgba(0,0,0,0.1)';
                                    if (categoryColor) {
                                      const hex = categoryColor.replace('#', '').padEnd(6, '0');
                                      const r = parseInt(hex.substring(0, 2), 16) || 0;
                                      const g = parseInt(hex.substring(2, 4), 16) || 0;
                                      const b = parseInt(hex.substring(4, 6), 16) || 0;
                                      const brightness = (r * 299 + g * 587 + b * 114) / 1000;
                                      textColor = brightness > 128 ? '#000000' : '#ffffff';
                                      borderColor = brightness > 128 ? 'rgba(0,0,0,0.1)' : 'rgba(255,255,255,0.2)';
                                    }

                                    // Find who didn't pay (for the split bar)
                                    const otherMembers = memberIds.filter((id) => id !== expense.payer_id);
                                    const payerShare = split[expense.payer_id] ?? 0;

                                    return (
                                      <div
                                        key={expense.id}
                                        className="px-2 py-1.5 rounded-notion relative overflow-hidden"
                                        style={{
                                          backgroundColor: categoryColor ? categoryColor + '15' : undefined,
                                          color: textColor,
                                        }}
                                      >
                                        <div className="flex items-center justify-between gap-2 mb-1">
                                          <div className="flex-1 min-w-0 flex items-center gap-1.5">
                                            {category?.icon && <span className="text-[10px] shrink-0">{category.icon}</span>}
                                            <div className="min-w-0 flex-1">
                                              <div className="flex items-center gap-1.5 flex-wrap">
                                                {expense.vendor && (
                                                  <span className="text-[11px] font-medium line-clamp-1">
                                                    {expense.vendor}
                                                  </span>
                                                )}
                                                {expense.vendor && expense.description && (
                                                  <span className="text-[10px] opacity-70">•</span>
                                                )}
                                                {expense.description && (
                                                  <span className="text-[11px] font-medium line-clamp-1 opacity-90">
                                                    {expense.description}
                                                  </span>
                                                )}
                                                {!expense.vendor && !expense.description && (
                                                  <span className="text-[11px] font-medium">Transaction</span>
                                                )}
                                                {category && (
                                                  <span className="text-[10px] opacity-70 shrink-0">
                                                    • {category.name}
                                                  </span>
                                                )}
                                              </div>
                                              <div className="flex items-center gap-1.5 mt-0.5">
                                                <span className="text-[10px] opacity-70">{formattedDate}</span>
                                                <span className="text-[10px] opacity-70">•</span>
                                                <span className="text-[10px] opacity-70">
                                                  Paid by <span className="font-medium">{payerName}</span>
                                                </span>
                                              </div>
                                            </div>
                                          </div>
                                          <div className="text-right shrink-0">
                                            <div className="text-xs font-bold tabular-nums">{formatCurrency(expense.amount)}</div>
                                          </div>
                                        </div>

                                        {/* Split bar at bottom - payer on left, others on right */}
                                        <div className="relative h-4 rounded-notion overflow-hidden" style={{ backgroundColor: categoryColor ? categoryColor + '20' : 'rgba(0,0,0,0.05)' }}>
                                          {/* Payer's portion on the left */}
                                          {payerShare > 0 && (
                                            <div
                                              className="absolute top-0 bottom-0 left-0 flex items-center pl-1"
                                              style={{
                                                width: `${payerShare}%`,
                                                backgroundColor: categoryColor ? categoryColor + '50' : 'rgba(0,0,0,0.1)',
                                              }}
                                            >
                                              <span className="text-[9px] font-medium tabular-nums whitespace-nowrap" style={{ color: textColor, opacity: 0.9 }}>
                                                {payerName}
                                              </span>
                                            </div>
                                          )}
                                          
                                          {/* Other members' portions from right */}
                                          {otherMembers.map((memberId, idx) => {
                                            const percentage = split[memberId] ?? 0;
                                            if (percentage === 0) return null;
                                            
                                            const share = (expense.amount * percentage) / 100;
                                            const memberName = memberNameMap.get(memberId) || 'Unknown';
                                            
                                            // Calculate position from right
                                            const otherPercentages = otherMembers.map((id) => split[id] ?? 0);
                                            const afterThis = otherPercentages.slice(idx + 1).reduce((sum, pct) => sum + pct, 0);
                                            const rightPosition = afterThis;
                                            const width = percentage;

                                            return (
                                              <div
                                                key={memberId}
                                                className="absolute top-0 bottom-0 flex items-center justify-end pr-1"
                                                style={{
                                                  right: `${rightPosition}%`,
                                                  width: `${width}%`,
                                                  backgroundColor: categoryColor ? categoryColor + '60' : 'rgba(0,0,0,0.15)',
                                                }}
                                              >
                                                <span className="text-[9px] font-medium tabular-nums whitespace-nowrap" style={{ color: textColor }}>
                                                  {memberName} pays: {formatCurrency(share)}
                                                </span>
                                              </div>
                                            );
                                          })}
                                        </div>
                                      </div>
                                    );
                                  })}
                                </div>
                              </div>
                            )}
                          </div>
                        );
                      })}
                  </div>
                )}
              </CardContent>
            </Card>
          </>
        )}
      </div>
    </MainLayout>
  );
}

