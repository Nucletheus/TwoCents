'use client';

import { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import {
  DataEditor,
  GridCellKind,
  getDefaultTheme,
  withAlpha,
  type GridCell,
  type GridColumn,
  type Item,
  type GridSelection,
  CompactSelection,
  type EditableGridCell,
  type EditListItem,
  type Theme as GlideTheme,
} from '@glideapps/glide-data-grid';
import '@glideapps/glide-data-grid/dist/index.css';
import { createClient } from '@/lib/supabase/client';
import type { Expense, Category } from '@twocents/shared';
import { formatCurrency, formatDate, generateColorVariations, getCategoryColor } from '@twocents/shared';
import { useTheme as useAppTheme } from '../components/ThemeProvider';
import GridDateEditor from '../components/GridDateEditor';
import GridDropdownEditor, { type DropdownOption } from '../components/GridDropdownEditor';
import ExpenseComments from './ExpenseComments';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog';
import { Check, X, MessageSquare, AlertCircle, CheckCircle2 } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Button from '../components/ui/Button';

type TwocentsEditorKind = 'date' | 'category' | 'payer' | 'account';

type TwocentsCellMeta = {
  twocentsEditor?: TwocentsEditorKind;
};

type TwocentsGridCell = GridCell & TwocentsCellMeta;

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

interface ReviewGridProps {
  householdId: string | null;
  categories: Category[];
  householdMembers: HouseholdMember[];
  accounts: Account[];
  onExpenseUpdated: () => void;
  transactionsFlaggedCounter?: number;
}

function scheduleMicrotask(cb: () => void) {
  if (typeof queueMicrotask === 'function') queueMicrotask(cb);
  else Promise.resolve().then(cb);
}

export default function ReviewGrid({
  householdId,
  categories,
  householdMembers,
  accounts,
  onExpenseUpdated,
  transactionsFlaggedCounter,
}: ReviewGridProps) {
  const supabase = createClient();
  const { theme: appTheme, palette } = useAppTheme();
  const [pendingExpenses, setPendingExpenses] = useState<Expense[]>([]);
  const [loading, setLoading] = useState(true);
  const [commentsDialogOpen, setCommentsDialogOpen] = useState(false);
  const [selectedExpenseForComments, setSelectedExpenseForComments] = useState<string | null>(null);
  const [currentUserId, setCurrentUserId] = useState<string | null>(null);
  const [originalFlaggers, setOriginalFlaggers] = useState<Record<string, string>>({});

  useEffect(() => {
    const fetchCurrentUser = async () => {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      setCurrentUserId(user?.id || null);
    };
    fetchCurrentUser();
  }, [supabase]);
  const [gridSelection, setGridSelection] = useState<GridSelection>(() => ({
    columns: CompactSelection.empty(),
    rows: CompactSelection.empty(),
  }));
  const gridRef = useRef<HTMLDivElement>(null);

  const glideTheme = useMemo<Partial<GlideTheme>>(() => {
    if (typeof window === 'undefined') return {};

    const root = document.documentElement;
    const cs = getComputedStyle(root);
    const getVar = (name: string) => cs.getPropertyValue(name).trim();

    const defaults = getDefaultTheme();

    const background = getVar('--background') || (appTheme === 'dark' ? '#191919' : '#ffffff');
    const foreground = getVar('--foreground') || (appTheme === 'dark' ? '#ffffff' : '#37352f');
    const muted = getVar('--muted') || (appTheme === 'dark' ? '#2e2e2e' : '#f7f6f3');
    const mutedFg = getVar('--muted-foreground') || (appTheme === 'dark' ? '#9b9a97' : '#787774');
    const border = getVar('--border') || (appTheme === 'dark' ? '#3d3d3d' : '#e9e9e7');
    const hover = getVar('--hover') || (appTheme === 'dark' ? '#373737' : '#f1f1ef');
    const card = getVar('--card') || muted || background;
    const accent = getVar('--accent') || defaults.accentColor;
    const accentFg = getVar('--accent-foreground') || defaults.accentFg;

    return {
      accentColor: accent,
      accentFg,
      accentLight: withAlpha(accent, 0.12),

      textDark: foreground,
      textMedium: mutedFg,
      textLight: withAlpha(mutedFg, 0.7),
      textBubble: foreground,

      bgIconHeader: mutedFg,
      fgIconHeader: accentFg,
      textHeader: foreground,
      textGroupHeader: mutedFg,
      textHeaderSelected: accentFg,

      bgCell: card,
      bgCellMedium: muted,
      bgHeader: muted,
      bgHeaderHasFocus: hover,
      bgHeaderHovered: hover,
      bgBubble: muted,
      bgBubbleSelected: card,
      bgSearchResult: withAlpha(accent, appTheme === 'dark' ? 0.22 : 0.14),

      borderColor: border,
      horizontalBorderColor: border,
      headerBottomBorderColor: border,

      linkColor: accent,
    };
  }, [appTheme, palette]);

  const fetchPendingExpenses = useCallback(async () => {
    if (!householdId) return;

    setLoading(true);
    try {
      // First, get all expenses that are pending/flagged OR approved with unresolved flags
      // We need to do this in two queries because we need to check for unresolved flags
      
      // Get pending/flagged expenses
      const { data: pendingData, error: pendingError } = await supabase
        .from('expenses')
        .select(
          `
          *,
          categories (
            id,
            name,
            icon,
            color,
            group_name
          )
        `
        )
        .eq('household_id', householdId)
        .in('status', ['pending_review', 'flagged']);

      if (pendingError) throw pendingError;

      // Get approved expenses that have unresolved flags
      const { data: flagsData, error: flagsError } = await supabase
        .from('expense_flags')
        .select('expense_id, user_id, created_at')
        .eq('flag_type', 'review')
        .is('resolved_at', null);

      if (flagsError) throw flagsError;

      // Get expense IDs with unresolved flags
      const expenseIdsWithUnresolvedFlags = new Set(
        (flagsData || []).map(f => f.expense_id)
      );

      // Get approved or dismissed expenses that have unresolved flags
      let reviewedData: any[] = [];
      if (expenseIdsWithUnresolvedFlags.size > 0) {
        const { data: reviewed, error: reviewedError } = await supabase
          .from('expenses')
          .select(
            `
            *,
            categories (
              id,
              name,
              icon,
              color,
              group_name
            )
          `
          )
          .eq('household_id', householdId)
          .in('status', ['approved', 'dismissed'])
          .in('id', Array.from(expenseIdsWithUnresolvedFlags));

        if (reviewedError) throw reviewedError;
        reviewedData = reviewed || [];
      }

      // Combine and sort: pending/flagged first, then approved/dismissed at bottom
      const allExpenses = [
        ...(pendingData || []),
        ...reviewedData,
      ];

      // Sort: pending/flagged first (by date desc), then approved/dismissed (by reviewed_at desc)
      allExpenses.sort((a, b) => {
        const aIsPending = a.status === 'pending_review' || a.status === 'flagged';
        const bIsPending = b.status === 'pending_review' || b.status === 'flagged';
        
        if (aIsPending && !bIsPending) return -1;
        if (!aIsPending && bIsPending) return 1;
        
        // Both same type, sort by date or reviewed_at
        if (aIsPending) {
          return new Date(b.date).getTime() - new Date(a.date).getTime();
        } else {
          const aTime = a.reviewed_at ? new Date(a.reviewed_at).getTime() : 0;
          const bTime = b.reviewed_at ? new Date(b.reviewed_at).getTime() : 0;
          return bTime - aTime;
        }
      });

      setPendingExpenses(allExpenses);

      // Build original flagger map: expense_id -> user_id of first flagger
      // Sort flags by created_at to get the oldest (first) flagger for each expense
      const sortedFlags = [...(flagsData || [])].sort((a, b) => 
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      );
      
      const flaggerMap: Record<string, string> = {};
      sortedFlags.forEach(flag => {
        if (!flaggerMap[flag.expense_id]) {
          // Store the first (oldest) flagger for each expense
          flaggerMap[flag.expense_id] = flag.user_id;
        }
      });
      setOriginalFlaggers(flaggerMap);
    } catch (error) {
      console.error('Error fetching pending expenses:', error);
    } finally {
      setLoading(false);
    }
  }, [householdId, supabase]);

  useEffect(() => {
    if (householdId) {
      fetchPendingExpenses();
    }
  }, [householdId, fetchPendingExpenses]);

  // Listen for when transactions are flagged and refresh after a delay
  useEffect(() => {
    if (transactionsFlaggedCounter === undefined || transactionsFlaggedCounter === 0) return;

    // Refresh after a delay to allow database writes to complete
    const timeoutId = setTimeout(() => {
      fetchPendingExpenses();
    }, 750); // 750ms delay

    return () => clearTimeout(timeoutId);
  }, [transactionsFlaggedCounter, fetchPendingExpenses]);

  const getCategoryName = useCallback((categoryId: string) => {
    const category = categories.find((c) => c.id === categoryId);
    return category?.name || '';
  }, [categories]);

  const getMemberName = useCallback((userId: string) => {
    const member = householdMembers.find((m) => m.user_id === userId);
    return member?.profiles?.name || member?.profiles?.email || '';
  }, [householdMembers]);

  const getAccountName = useCallback((accountId: string | null) => {
    if (!accountId) return '';
    const account = accounts.find((a) => a.id === accountId);
    return account ? `${account.name} (${account.type})` : '';
  }, [accounts]);

  const categoryDropdownOptions = useMemo<DropdownOption[]>(() => {
    const groups: Record<string, Category[]> = {};
    const uncategorized: Category[] = [];

    for (const cat of categories) {
      if (cat.group_name) {
        if (!groups[cat.group_name]) groups[cat.group_name] = [];
        groups[cat.group_name].push(cat);
      } else {
        uncategorized.push(cat);
      }
    }

    const sortedGroups = Object.keys(groups).sort((a, b) => a.localeCompare(b));
    const out: DropdownOption[] = [];

    for (const groupName of sortedGroups) {
      const groupCategories = groups[groupName];
      const groupBaseColor =
        groupCategories.find((c) => c.parent_color)?.parent_color ||
        groupCategories.find((c) => c.color)?.color ||
        null;

      groupCategories.forEach((cat, idx) => {
        const color = groupBaseColor
          ? generateColorVariations(groupBaseColor, idx, groupCategories.length)
          : getCategoryColor(cat, idx, groupCategories.length);
        out.push({
          id: cat.id,
          label: cat.name,
          group: groupName,
          color,
          parentColor: groupBaseColor,
          icon: cat.icon,
        });
      });
    }

    uncategorized.forEach((cat, idx) => {
      out.push({
        id: cat.id,
        label: cat.name,
        group: 'Other',
        color: getCategoryColor(cat, idx, uncategorized.length),
        icon: cat.icon,
      });
    });

    return out;
  }, [categories]);

  const partnerDropdownOptions = useMemo<DropdownOption[]>(() => {
    return householdMembers.map((m) => ({
      id: m.user_id,
      label: m.profiles?.name || m.profiles?.email || 'Unknown',
      subLabel: m.profiles?.email || undefined,
    }));
  }, [householdMembers]);

  const accountDropdownOptions = useMemo<DropdownOption[]>(() => {
    const opts = accounts.map((a) => ({
      id: a.id,
      label: a.name,
      subLabel: a.type,
    }));
    return [{ id: '', label: 'No account' }, ...opts];
  }, [accounts]);

  const provideEditor = useCallback(
    (cell: GridCell) => {
      const editor = (cell as TwocentsGridCell).twocentsEditor;
      if (!editor) return undefined;

      if (editor === 'date') {
        return GridDateEditor as any;
      }

      if (editor === 'category') {
        if (categoryDropdownOptions.length === 0) return undefined;
        return (p: any) => (
          <GridDropdownEditor
            {...p}
            title="Category"
            options={categoryDropdownOptions}
            placeholder="Search categories…"
          />
        );
      }

      if (editor === 'payer') {
        if (partnerDropdownOptions.length === 0) return undefined;
        return (p: any) => (
          <GridDropdownEditor
            {...p}
            title="Partner"
            options={partnerDropdownOptions}
            placeholder="Search partners…"
          />
        );
      }

      if (editor === 'account') {
        return (p: any) => (
          <GridDropdownEditor
            {...p}
            title="Account"
            options={accountDropdownOptions}
            placeholder="Search accounts…"
          />
        );
      }

      return undefined;
    },
    [accountDropdownOptions, categoryDropdownOptions, partnerDropdownOptions]
  );

  const computeUpdateForCell = useCallback(
    (col: number, expense: Expense, newValue: EditableGridCell) => {
      let updateData: Partial<Expense> | null = null;
      let nextExpense: Expense | null = null;
      const getText = () => String((newValue as any).data ?? '').trim();

      switch (col) {
        case 0: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const nextDate = getText();
          if (!nextDate) return null;
          updateData = { date: nextDate };
          nextExpense = { ...expense, date: nextDate };
          break;
        }
        case 1: {
          const raw = (newValue as any).data;
          const nextAmount = typeof raw === 'number' ? raw : Number(raw);
          if (!Number.isFinite(nextAmount)) return null;
          // Amounts are stored as absolute values in the DB (constraint: amount >= 0)
          const normalizedAmount = Math.abs(nextAmount);
          updateData = { amount: normalizedAmount };
          nextExpense = { ...expense, amount: normalizedAmount };
          break;
        }
        case 2: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const input = getText();
          if (!input) return null;
          const match =
            categories.find((c) => c.id === input) ??
            categories.find((c) => c.name.toLowerCase() === input.toLowerCase());
          if (!match) return null;
          updateData = { category_id: match.id };
          nextExpense = { ...expense, category_id: match.id };
          break;
        }
        case 3: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const input = String((newValue as any).data ?? '');
          const nextDesc = input.trim() === '' ? null : input;
          updateData = { description: nextDesc };
          nextExpense = { ...expense, description: nextDesc };
          break;
        }
        case 4: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const raw = getText();
          const input = raw.toLowerCase();
          if (!raw) return null;
          const match = householdMembers.find((m) => {
            const label = (m.profiles?.name || m.profiles?.email || '').toLowerCase();
            return m.user_id === raw || label === input;
          });
          if (!match) return null;
          updateData = { payer_id: match.user_id };
          nextExpense = { ...expense, payer_id: match.user_id };
          break;
        }
        case 5: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const raw = getText();
          const input = raw.toLowerCase();
          if (!raw) {
            updateData = { account_id: null };
            nextExpense = { ...expense, account_id: null };
            break;
          }
          const match =
            accounts.find((a) => a.id === raw) ??
            accounts.find((a) => a.name.toLowerCase() === input || `${a.name} (${a.type})`.toLowerCase() === input);
          if (!match) return null;
          updateData = { account_id: match.id };
          nextExpense = { ...expense, account_id: match.id };
          break;
        }
        default:
          return null;
      }

      if (!updateData || !nextExpense) return null;
      return { updateData, nextExpense };
    },
    [accounts, categories, householdMembers]
  );

  const handleCellsEdited = useCallback(
    (items: readonly EditListItem[]) => {
      if (items.length === 0) return true;

      const rowPatches: Array<{ realIdx: number; nextExpense: Expense }> = [];
      const updatesById = new Map<string, Partial<Expense>>();

      for (const item of items) {
        const [col, rowIdx] = item.location;
        // All columns are editable now (no action columns)
        const expense = pendingExpenses[rowIdx];
        if (!expense) continue;

        const computed = computeUpdateForCell(col, expense, item.value);
        if (!computed) continue;

        rowPatches.push({ realIdx: rowIdx, nextExpense: computed.nextExpense });

        const prev = updatesById.get(expense.id) ?? {};
        updatesById.set(expense.id, { ...prev, ...computed.updateData });
      }

      if (updatesById.size === 0) return true;

      scheduleMicrotask(async () => {
        // Apply local UI updates
        setPendingExpenses((prevRows) => {
          if (rowPatches.length === 0) return prevRows;
          const next = [...prevRows];
          for (const p of rowPatches) {
            if (p.realIdx >= 0 && p.realIdx < next.length) {
              next[p.realIdx] = p.nextExpense;
            }
          }
          return next;
        });

        // Persist updates
        for (const [id, data] of updatesById.entries()) {
          try {
            const { error } = await supabase
              .from('expenses')
              .update(data)
              .eq('id', id);
            if (error) throw error;
          } catch (error) {
            console.error('Error updating expense:', error);
          }
        }
      });

      return true;
    },
    [computeUpdateForCell, pendingExpenses, supabase]
  );

  const handleApprove = useCallback(async (expenseId: string) => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { error } = await supabase
        .from('expenses')
        .update({
          status: 'approved',
          reviewed_by: user.id,
          reviewed_at: new Date().toISOString(),
        })
        .eq('id', expenseId);

      if (error) throw error;

      // DO NOT resolve flags - keep them unresolved so transaction stays in review list
      // DO NOT delete comments - keep chat history visible

      await fetchPendingExpenses();
      onExpenseUpdated();
    } catch (error) {
      console.error('Error approving expense:', error);
    }
  }, [supabase, onExpenseUpdated, fetchPendingExpenses]);

  const handleDismiss = useCallback(async (expenseId: string) => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { error } = await supabase
        .from('expenses')
        .update({
          status: 'dismissed',
          reviewed_by: user.id,
          reviewed_at: new Date().toISOString(),
        })
        .eq('id', expenseId);

      if (error) throw error;

      // DO NOT resolve flags - keep them unresolved so transaction stays in review list
      // DO NOT delete comments - keep chat history visible

      await fetchPendingExpenses();
      onExpenseUpdated();
    } catch (error) {
      console.error('Error dismissing expense:', error);
    }
  }, [supabase, onExpenseUpdated, fetchPendingExpenses]);

  const handleBulkApprove = useCallback(async () => {
    if (!gridSelection.rows || gridSelection.rows.length === 0) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const selectedIndices = Array.from(gridSelection.rows);
      const selectedExpenses = selectedIndices
        .map((idx) => pendingExpenses[idx])
        .filter(Boolean);

      const expenseIds = selectedExpenses.map((e) => e.id);
      const now = new Date().toISOString();

      // Update all expenses to approved status
      const { error: updateError } = await supabase
        .from('expenses')
        .update({
          status: 'approved',
          reviewed_by: user.id,
          reviewed_at: now,
        })
        .in('id', expenseIds);

      if (updateError) throw updateError;

      // DO NOT resolve flags - keep them unresolved so transactions stay in review list
      // DO NOT delete comments - keep chat history visible

      await fetchPendingExpenses();
      onExpenseUpdated();

      setGridSelection({
        columns: CompactSelection.empty(),
        rows: CompactSelection.empty(),
      });
    } catch (error) {
      console.error('Error bulk approving expenses:', error);
    }
  }, [gridSelection.rows, pendingExpenses, supabase, onExpenseUpdated, fetchPendingExpenses]);

  const handleBulkDismiss = useCallback(async () => {
    if (!gridSelection.rows || gridSelection.rows.length === 0) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const selectedIndices = Array.from(gridSelection.rows);
      const selectedExpenses = selectedIndices
        .map((idx) => pendingExpenses[idx])
        .filter(Boolean);

      const expenseIds = selectedExpenses.map((e) => e.id);
      const now = new Date().toISOString();

      // Update all expenses to dismissed status
      const { error: updateError } = await supabase
        .from('expenses')
        .update({
          status: 'dismissed',
          reviewed_by: user.id,
          reviewed_at: now,
        })
        .in('id', expenseIds);

      if (updateError) throw updateError;

      // DO NOT resolve flags - keep them unresolved so transactions stay in review list
      // DO NOT delete comments - keep chat history visible

      await fetchPendingExpenses();
      onExpenseUpdated();

      setGridSelection({
        columns: CompactSelection.empty(),
        rows: CompactSelection.empty(),
      });
    } catch (error) {
      console.error('Error bulk dismissing expenses:', error);
    }
  }, [gridSelection.rows, pendingExpenses, supabase, onExpenseUpdated, fetchPendingExpenses]);

  const handleClear = useCallback(async () => {
    if (!gridSelection.rows || gridSelection.rows.length === 0) return;
    if (!currentUserId) return;

    try {
      const selectedIndices = Array.from(gridSelection.rows);
      const selectedExpenses = selectedIndices
        .map((idx) => pendingExpenses[idx])
        .filter(Boolean);

      // Filter to only expenses where current user is the original flagger
      const clearableExpenses = selectedExpenses.filter(expense => {
        return originalFlaggers[expense.id] === currentUserId;
      });

      if (clearableExpenses.length === 0) {
        // No clearable expenses - user is not the original flagger
        return;
      }

      const expenseIds = clearableExpenses.map((e) => e.id);
      const now = new Date().toISOString();

      // Resolve all flags for these expenses (this removes them from review list)
      await supabase
        .from('expense_flags')
        .update({ resolved_at: now })
        .in('expense_id', expenseIds)
        .eq('flag_type', 'review')
        .is('resolved_at', null);

      // Clear all comments for these expenses
      await supabase
        .from('expense_comments')
        .delete()
        .in('expense_id', expenseIds);

      await fetchPendingExpenses();
      onExpenseUpdated();

      setGridSelection({
        columns: CompactSelection.empty(),
        rows: CompactSelection.empty(),
      });
    } catch (error) {
      console.error('Error clearing expenses:', error);
    }
  }, [gridSelection.rows, pendingExpenses, currentUserId, originalFlaggers, supabase, onExpenseUpdated, fetchPendingExpenses]);

  const handleViewComments = useCallback(() => {
    if (!gridSelection.rows || gridSelection.rows.length === 0) return;
    
    // Only allow comments for single selection
    const selectedIndices = Array.from(gridSelection.rows);
    if (selectedIndices.length !== 1) return;

    const expense = pendingExpenses[selectedIndices[0]];
    if (!expense) return;

    setSelectedExpenseForComments(expense.id);
    setCommentsDialogOpen(true);
  }, [gridSelection.rows, pendingExpenses]);

  // Get theme override for approved/dismissed rows (distinct backgrounds)
  const getRowThemeOverride = useCallback((expense: Expense): Partial<GlideTheme> | undefined => {
    if (expense.status === 'approved' && expense.reviewed_by) {
      // Approved rows get a distinct green background
      return {
        bgCell: appTheme === 'dark' ? withAlpha('#10b981', 0.25) : withAlpha('#10b981', 0.15),
        bgCellMedium: appTheme === 'dark' ? withAlpha('#10b981', 0.20) : withAlpha('#10b981', 0.12),
      };
    }
    if (expense.status === 'dismissed' && expense.reviewed_by) {
      // Dismissed rows get a distinct orange/amber background
      return {
        bgCell: appTheme === 'dark' ? withAlpha('#f59e0b', 0.25) : withAlpha('#f59e0b', 0.15),
        bgCellMedium: appTheme === 'dark' ? withAlpha('#f59e0b', 0.20) : withAlpha('#f59e0b', 0.12),
      };
    }
    return undefined;
  }, [appTheme]);

  const getCellContent = useCallback((cell: Item): GridCell => {
    const [col, rowIdx] = cell;
    const row = pendingExpenses[rowIdx];

    if (!row) {
      return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
    }

    const rowThemeOverride = getRowThemeOverride(row);

    switch (col) {
      case 0:
        return {
          kind: GridCellKind.Text,
          data: row.date,
          displayData: formatDate(row.date),
          allowOverlay: true,
          twocentsEditor: 'date',
          themeOverride: rowThemeOverride,
        } as TwocentsGridCell;
      case 1:
        return {
          kind: GridCellKind.Number,
          data: row.amount,
          displayData: formatCurrency(row.amount),
          allowOverlay: true,
          themeOverride: rowThemeOverride,
        };
      case 2: {
        const name = getCategoryName(row.category_id);
        return {
          kind: GridCellKind.Text,
          data: row.category_id,
          displayData: name,
          allowOverlay: true,
          twocentsEditor: 'category',
          themeOverride: rowThemeOverride,
        } as TwocentsGridCell;
      }
      case 3:
        return {
          kind: GridCellKind.Text,
          data: row.description ?? '',
          displayData: row.description ?? '',
          allowOverlay: true,
          themeOverride: rowThemeOverride,
        };
      case 4: {
        const name = getMemberName(row.payer_id);
        return {
          kind: GridCellKind.Text,
          data: row.payer_id,
          displayData: name,
          allowOverlay: true,
          twocentsEditor: 'payer',
          themeOverride: rowThemeOverride,
        } as TwocentsGridCell;
      }
      case 5: {
        const name = getAccountName(row.account_id);
        return {
          kind: GridCellKind.Text,
          data: row.account_id ?? '',
          displayData: name,
          allowOverlay: true,
          twocentsEditor: 'account',
          themeOverride: rowThemeOverride,
        } as TwocentsGridCell;
      }
      case 6: {
        // Status column - show who approved/dismissed
        if (row.status === 'approved' && row.reviewed_by) {
          const reviewerName = getMemberName(row.reviewed_by);
          const reviewedDate = row.reviewed_at ? formatDate(row.reviewed_at, 'MMM dd, HH:mm') : '';
          return {
            kind: GridCellKind.Text,
            data: `Approved by ${reviewerName}`,
            displayData: `✓ Approved by ${reviewerName}${reviewedDate ? ` (${reviewedDate})` : ''}`,
            allowOverlay: false,
            themeOverride: rowThemeOverride,
          };
        }
        if (row.status === 'dismissed' && row.reviewed_by) {
          const reviewerName = getMemberName(row.reviewed_by);
          const reviewedDate = row.reviewed_at ? formatDate(row.reviewed_at, 'MMM dd, HH:mm') : '';
          return {
            kind: GridCellKind.Text,
            data: `Dismissed by ${reviewerName}`,
            displayData: `✗ Dismissed by ${reviewerName}${reviewedDate ? ` (${reviewedDate})` : ''}`,
            allowOverlay: false,
            themeOverride: rowThemeOverride,
          };
        }
        return {
          kind: GridCellKind.Text,
          data: '',
          displayData: '',
          allowOverlay: false,
          themeOverride: rowThemeOverride,
        };
      }
      default:
        return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
    }
  }, [pendingExpenses, getCategoryName, getMemberName, getAccountName, getRowThemeOverride]);

  const glideColumns = useMemo<GridColumn[]>(() => {
    return [
      { id: 'date', title: 'Date', width: 120 },
      { id: 'amount', title: 'Amount', width: 120 },
      { id: 'category', title: 'Category', width: 180 },
      { id: 'description', title: 'Description', width: 260, grow: 1 },
      { id: 'payer', title: 'Partner', width: 180 },
      { id: 'account', title: 'Account', width: 220 },
      { id: 'status', title: 'Status', width: 200 },
    ];
  }, []);

  const selectedRowCount = useMemo(() => {
    if (!gridSelection.rows) return 0;
    return gridSelection.rows.length;
  }, [gridSelection.rows]);

  const canViewComments = selectedRowCount === 1;

  // Check if selected rows can be cleared (approved/dismissed and current user is original flagger)
  const canClearSelected = useMemo(() => {
    if (!gridSelection.rows || gridSelection.rows.length === 0 || !currentUserId) return false;
    
    const selectedIndices = Array.from(gridSelection.rows);
    const selectedExpenses = selectedIndices
      .map((idx) => pendingExpenses[idx])
      .filter(Boolean);

    // All selected expenses must be approved/dismissed
    const allApprovedOrDismissed = selectedExpenses.every(expense => 
      (expense.status === 'approved' || expense.status === 'dismissed') && expense.reviewed_by
    );

    if (!allApprovedOrDismissed) return false;

    // Current user must be the original flagger for all selected expenses
    const allClearable = selectedExpenses.every(expense => 
      originalFlaggers[expense.id] === currentUserId
    );

    return allClearable;
  }, [gridSelection.rows, pendingExpenses, currentUserId, originalFlaggers]);

  const rowHeight = 36;
  const headerHeight = 36;
  const dataEditorHeight = useMemo(() => {
    // Exact height: header + rows (no extra cushion)
    return headerHeight + pendingExpenses.length * rowHeight;
  }, [pendingExpenses.length]);

  if (loading) {
    return (
      <Card>
        <CardContent className="p-4">
          <div className="text-center text-muted-foreground">Loading pending reviews...</div>
        </CardContent>
      </Card>
    );
  }

  if (pendingExpenses.length === 0) {
    return null;
  }

  return (
    <>
      <Card className="border-accent/20 bg-accent/5">
        <CardHeader className="px-3 py-1.5">
          <CardTitle className="flex items-center gap-1.5 text-xs leading-tight">
            <AlertCircle className="h-3.5 w-3.5 text-accent" />
            Pending Review ({pendingExpenses.length})
          </CardTitle>
        </CardHeader>
        <CardContent className="px-3 pt-0 pb-2">
          {/* Action Toolbar */}
          <div className="flex items-center gap-2 mb-1.5 pb-1.5 border-b border-border">
            <div className="flex items-center gap-2 flex-1">
              <span className="text-xs text-muted-foreground">
                {selectedRowCount > 0 ? `${selectedRowCount} selected` : 'Select rows to perform actions'}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="ghost"
                onClick={handleViewComments}
                disabled={!canViewComments}
                className="h-7 text-xs"
                title={canViewComments ? 'View comments for selected expense' : 'Select a single expense to view comments'}
              >
                <MessageSquare className="h-3.5 w-3.5 mr-1" />
                Comments
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={handleBulkApprove}
                disabled={selectedRowCount === 0}
                className="h-7 text-xs text-green-600 hover:text-green-700"
                title="Approve selected expenses"
              >
                <Check className="h-3.5 w-3.5 mr-1" />
                Approve
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={handleBulkDismiss}
                disabled={selectedRowCount === 0}
                className="h-7 text-xs text-destructive"
                title="Dismiss selected expenses from review"
              >
                <X className="h-3.5 w-3.5 mr-1" />
                Dismiss
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={handleClear}
                disabled={!canClearSelected}
                className="h-7 text-xs text-blue-600 hover:text-blue-700"
                title={canClearSelected ? 'Clear approved/dismissed expenses (only original flagger can clear)' : 'Select approved/dismissed expenses that you originally flagged to clear them'}
              >
                <CheckCircle2 className="h-3.5 w-3.5 mr-1" />
                Clear
              </Button>
            </div>
          </div>
          <div ref={gridRef} className="w-full overflow-hidden relative">
            <DataEditor
              columns={glideColumns}
              rows={pendingExpenses.length}
              getCellContent={getCellContent}
              onCellsEdited={handleCellsEdited}
              provideEditor={provideEditor}
              theme={glideTheme}
              getCellsForSelection={true}
              gridSelection={gridSelection}
              onGridSelectionChange={setGridSelection}
              rowMarkers="both"
              width="100%"
              height={dataEditorHeight}
              rowHeight={rowHeight}
              headerHeight={headerHeight}
              cellActivationBehavior="single-click"
              editOnType={true}
            />
          </div>
        </CardContent>
      </Card>

      <Dialog open={commentsDialogOpen} onOpenChange={setCommentsDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh] flex flex-col p-0">
          <DialogHeader className="px-6 pt-6 pb-4 border-b border-border">
            <DialogTitle>Comments</DialogTitle>
            <DialogDescription>View and add comments for this expense</DialogDescription>
          </DialogHeader>
          <div className="flex-1 min-h-0">
            {selectedExpenseForComments && (() => {
              const expense = pendingExpenses.find(e => e.id === selectedExpenseForComments);
              return (
                <ExpenseComments 
                  expenseId={selectedExpenseForComments}
                  householdId={householdId}
                  currentUserId={currentUserId}
                  expenseStatus={expense?.status}
                  reviewedBy={expense?.reviewed_by}
                  reviewedAt={expense?.reviewed_at}
                  householdMembers={householdMembers}
                  onApprove={handleApprove}
                  onDismiss={handleDismiss}
                  onStatusUpdate={async () => {
                    await fetchPendingExpenses();
                  }}
                />
              );
            })()}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

