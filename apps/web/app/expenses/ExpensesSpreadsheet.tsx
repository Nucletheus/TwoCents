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
  type Rectangle,
} from '@glideapps/glide-data-grid';
import '@glideapps/glide-data-grid/dist/index.css';
import type { Expense, Category } from '@twocents/shared';
import { extractVendor, formatCurrency, formatDate, generateColorVariations, getCategoryColor, normalizeVendor } from '@twocents/shared';
import { createPortal } from 'react-dom';
import { Flag, BarChart3, Trash2 } from 'lucide-react';
import Button from '../components/ui/Button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../components/ui/dialog';
import Input from '../components/ui/Input';
import { useTheme as useAppTheme } from '../components/ThemeProvider';
import { useRouter } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import GridDateEditor from '../components/GridDateEditor';
import GridDropdownEditor, { type DropdownOption } from '../components/GridDropdownEditor';
import FlagCommentDialog from './FlagCommentDialog';

type TwocentsEditorKind = 'date' | 'category' | 'payer' | 'account';

type TwocentsCellMeta = {
  twocentsEditor?: TwocentsEditorKind;
};

type TwocentsGridCell = GridCell & TwocentsCellMeta;

function scheduleMicrotask(cb: () => void) {
  // `queueMicrotask` is widely supported, but provide a safe fallback.
  if (typeof queueMicrotask === 'function') queueMicrotask(cb);
  else Promise.resolve().then(cb);
}


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

interface ExpensesSpreadsheetProps {
  expenses: Expense[];
  categories: Category[];
  householdMembers: HouseholdMember[];
  accounts: Account[];
  toolbarPortalEl?: HTMLElement | null;
  onUpdate: (id: string, data: Partial<Expense>) => Promise<void>;
  onBulkUpdate?: (updates: Array<{ id: string; data: Partial<Expense> }>) => Promise<void>;
  onBulkDelete?: (ids: string[]) => Promise<void>;
  expenseFlags?: Record<string, boolean>;
  onFlagChange?: () => void;
  onTransactionsFlagged?: () => void;
  householdId?: string | null;
  userColors?: Record<string, string>;
  isFixedMode?: boolean; // When true, grid fills remaining viewport space
}

type GridRow = Expense;

export default function ExpensesSpreadsheet({
  expenses,
  categories,
  householdMembers,
  accounts,
  toolbarPortalEl,
  onUpdate,
  onBulkUpdate,
  onBulkDelete,
  expenseFlags = {},
  onFlagChange,
  onTransactionsFlagged,
  householdId,
  userColors = {},
  isFixedMode = false,
}: ExpensesSpreadsheetProps) {
  const { theme: appTheme } = useAppTheme();
  const router = useRouter();
  const supabase = createClient();
  const [rows, setRows] = useState<GridRow[]>([]);
  const [filterValue, setFilterValue] = useState('');
  const [debouncedFilterValue, setDebouncedFilterValue] = useState('');
  const [showSearch, setShowSearch] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [showFlagCommentDialog, setShowFlagCommentDialog] = useState(false);
  const [pendingFlagExpenseIds, setPendingFlagExpenseIds] = useState<string[]>([]);
  const [gridSelection, setGridSelection] = useState<GridSelection>(() => ({
    columns: CompactSelection.empty(),
    rows: CompactSelection.empty(),
  }));
  const gridRef = useRef<HTMLDivElement>(null);
  
  // Local update tracking: Prevents props from overwriting local UI changes before they're persisted
  // When a user edits a cell, we update local state immediately for instant feedback. However, if the parent
  // component refetches data before our update completes, the props would overwrite our local change.
  // This Set tracks which expense IDs have pending local updates, so we can preserve them during prop updates.
  const locallyUpdatedExpenseIds = useRef<Set<string>>(new Set());
  
  // Undo/redo system for unpersisted changes
  // undoHistory: Stores previous state (before change) for reverting with Ctrl+Z
  // redoHistory: Stores next state (after change) for reapplying with Ctrl+Y or Ctrl+Shift+Z
  // History is limited to MAX_HISTORY_SIZE entries and cleared when changes are persisted to the database
  const [undoHistory, setUndoHistory] = useState<Array<{ expenseId: string; previousExpense: Expense }>>([]);
  const [redoHistory, setRedoHistory] = useState<Array<{ expenseId: string; nextExpense: Expense }>>([]);
  const MAX_HISTORY_SIZE = 50;
  
  // Sort state with localStorage persistence - remembers user's sort preferences across sessions
  // Defaults to sorting by date descending (newest first)
  const STORAGE_KEY = 'twocents:expenses-sort';
  const [sortColumn, setSortColumn] = useState<string>('date');
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('desc');
  
  // Load sort preference from localStorage on mount to restore user's previous sort settings
  useEffect(() => {
    if (typeof window === 'undefined') return;
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        if (parsed.column && typeof parsed.column === 'string') {
          setSortColumn(parsed.column);
        }
        if (parsed.direction === 'asc' || parsed.direction === 'desc') {
          setSortDirection(parsed.direction);
        }
      }
    } catch {
      // Keep defaults on error
    }
  }, []);
  
  // Save sort preference to localStorage
  useEffect(() => {
    if (typeof window === 'undefined') return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ column: sortColumn, direction: sortDirection }));
    } catch {
      // ignore storage errors
    }
  }, [sortColumn, sortDirection]);
  
  // Similar transactions dialog state - shown when user changes a category
  // The dialog helps users apply the same category to similar transactions (same vendor/description)
  // vendorOnlyMatches: Transactions with same vendor but different descriptions
  // vendorAndDescMatches: Transactions with same vendor AND description (stronger match)
  // pendingCategoryUpdate: The category change that triggered the dialog (stored until user confirms)
  const [showSimilarTransactionsDialog, setShowSimilarTransactionsDialog] = useState(false);
  const [vendorOnlyMatches, setVendorOnlyMatches] = useState<Expense[]>([]);
  const [vendorAndDescMatches, setVendorAndDescMatches] = useState<Expense[]>([]);
  const [pendingCategoryUpdate, setPendingCategoryUpdate] = useState<{ expenseId: string; categoryId: string } | null>(null);
  const [vendorOnlyGridSelection, setVendorOnlyGridSelection] = useState<GridSelection>(() => ({
    columns: CompactSelection.empty(),
    rows: CompactSelection.empty(),
  }));
  const [vendorAndDescGridSelection, setVendorAndDescGridSelection] = useState<GridSelection>(() => ({
    columns: CompactSelection.empty(),
    rows: CompactSelection.empty(),
  }));
  const [pendingRuleCreation, setPendingRuleCreation] = useState<{
    vendor: string | null;
    description: string | null;
    categoryId: string;
  } | null>(null);

  const glideTheme = useMemo<Partial<GlideTheme>>(() => {
    // Glide renders via canvas and won't understand `var(--...)` colors.
    // We read actual computed colors from our Tailwind CSS variables so it matches light/dark + palette.
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
  }, [appTheme]);

  // Debounce filter input for better performance
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedFilterValue(filterValue);
    }, 150);
    return () => clearTimeout(timer);
  }, [filterValue]);

  // Create lookup maps for O(1) access instead of O(n) .find()
  const categoryMap = useMemo(() => {
    const map = new Map<string, string>();
    categories.forEach((c) => {
      map.set(c.id, c.name);
    });
    return map;
  }, [categories]);

  const memberMap = useMemo(() => {
    const map = new Map<string, string>();
    householdMembers.forEach((m) => {
      const name = m.profiles?.name || m.profiles?.email || '';
      map.set(m.user_id, name);
    });
    return map;
  }, [householdMembers]);

  const accountMap = useMemo(() => {
    const map = new Map<string, string>();
    accounts.forEach((a) => {
      map.set(a.id, `${a.name} (${a.type})`);
    });
    return map;
  }, [accounts]);

  // Sort function (defined after maps are created)
  const sortRows = useCallback((rowsToSort: Expense[]): Expense[] => {
    if (!sortColumn || rowsToSort.length === 0) return rowsToSort;
    
    const sorted = [...rowsToSort].sort((a, b) => {
      let aVal: any;
      let bVal: any;
      
      switch (sortColumn) {
        case 'date':
          aVal = a.date;
          bVal = b.date;
          break;
        case 'amount':
          aVal = a.amount;
          bVal = b.amount;
          break;
        case 'category':
          aVal = categoryMap.get(a.category_id) || '';
          bVal = categoryMap.get(b.category_id) || '';
          break;
        case 'vendor':
          aVal = a.vendor || '';
          bVal = b.vendor || '';
          break;
        case 'description':
          aVal = a.description || '';
          bVal = b.description || '';
          break;
        case 'payer':
          aVal = memberMap.get(a.payer_id) || '';
          bVal = memberMap.get(b.payer_id) || '';
          break;
        case 'account':
          aVal = a.account_id ? (accountMap.get(a.account_id) || '') : '';
          bVal = b.account_id ? (accountMap.get(b.account_id) || '') : '';
          break;
        default:
          return 0;
      }
      
      // Handle null/undefined values
      if (aVal == null && bVal == null) return 0;
      if (aVal == null) return 1;
      if (bVal == null) return -1;
      
      // Compare values
      let comparison = 0;
      if (typeof aVal === 'number' && typeof bVal === 'number') {
        comparison = aVal - bVal;
      } else {
        comparison = String(aVal).localeCompare(String(bVal));
      }
      
      return sortDirection === 'asc' ? comparison : -comparison;
    });
    
    return sorted;
  }, [sortColumn, sortDirection, categoryMap, memberMap, accountMap]);

  // Update rows when expenses change, but preserve local changes that haven't been reflected yet
  // This effect merges new expense data from props with local UI changes, ensuring:
  // 1. Local edits aren't lost when parent refetches
  // 2. New expenses are added to the grid
  // 3. Updated expenses are reflected (once local changes are persisted)
  // 4. The grid is re-sorted after merging
  useEffect(() => {
    const effectStartTime = Date.now();
    // #region agent log
    if (typeof window !== 'undefined') {
      fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:useEffect expenses',message:'expenses effect triggered',data:{expensesCount:expenses.length},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'G'})}).catch(()=>{});
    }
    // #endregion
    
    setRows((prevRows) => {
      const setRowsStartTime = Date.now();
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:setRows',message:'setRows callback started',data:{prevRowsCount:prevRows.length,expensesCount:expenses.length},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'G'})}).catch(()=>{});
      }
      // #endregion
      
      // If we have no previous rows, just use the new expenses
      if (prevRows.length === 0) {
        const sortStartTime = Date.now();
        const result = sortRows(expenses);
        const sortEndTime = Date.now();
        // #region agent log
        if (typeof window !== 'undefined') {
          fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:setRows',message:'sortRows completed (no prev rows)',data:{sortTime:sortEndTime-sortStartTime,resultCount:result.length},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'G'})}).catch(()=>{});
        }
        // #endregion
        return result;
      }

      const mapStartTime = Date.now();
      // Create maps for quick lookup
      const newExpensesMap = new Map(expenses.map(e => [e.id, e]));
      const prevRowsMap = new Map(prevRows.map(e => [e.id, e]));
      const mapEndTime = Date.now();
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:setRows',message:'maps created',data:{mapTime:mapEndTime-mapStartTime},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'G'})}).catch(()=>{});
      }
      // #endregion
      
      const mergeStartTime = Date.now();
      // Merge: preserve local changes for expenses that were updated locally
      const merged = expenses.map((newExpense) => {
        const prevExpense = prevRowsMap.get(newExpense.id);
        
        // If this expense was updated locally, preserve the local version if category_id differs
        // (this means the parent refresh happened before our update completed)
        if (prevExpense && locallyUpdatedExpenseIds.current.has(newExpense.id)) {
          if (prevExpense.category_id !== newExpense.category_id) {
            // Local update hasn't been reflected in props yet, preserve it
            return prevExpense;
          } else {
            // Update has been reflected, remove from tracking
            locallyUpdatedExpenseIds.current.delete(newExpense.id);
          }
        }
        
        return newExpense;
      });
      const mergeEndTime = Date.now();
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:setRows',message:'merge completed',data:{mergeTime:mergeEndTime-mergeStartTime},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'G'})}).catch(()=>{});
      }
      // #endregion
      
      const filterStartTime = Date.now();
      // Only add "missing" expenses if expenses array is not empty
      // When expenses is intentionally empty (e.g., after bulk delete), don't add back prevRows
      // This prevents deleted expenses from reappearing
      let missing: Expense[] = [];
      if (expenses.length > 0) {
        // Only check for missing expenses when we have new expenses to merge
        // This handles cases where some expenses were updated but others weren't
        const newExpenseIds = new Set(expenses.map(e => e.id));
        missing = prevRows.filter(e => !newExpenseIds.has(e.id));
      }
      // If expenses.length === 0, we intentionally want an empty result (don't add back prevRows)
      const filterEndTime = Date.now();
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:setRows',message:'filter completed',data:{filterTime:filterEndTime-filterStartTime,missingCount:missing.length,expensesLength:expenses.length},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'G'})}).catch(()=>{});
      }
      // #endregion
      
      const finalSortStartTime = Date.now();
      const combined = [...merged, ...missing];
      const sorted = sortRows(combined);
      const finalSortEndTime = Date.now();
      const setRowsEndTime = Date.now();
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:setRows',message:'setRows callback completed',data:{finalSortTime:finalSortEndTime-finalSortStartTime,totalSetRowsTime:setRowsEndTime-setRowsStartTime,resultCount:sorted.length},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'G'})}).catch(()=>{});
      }
      // #endregion
      return sorted;
    });
    
    const effectEndTime = Date.now();
    // #region agent log
    if (typeof window !== 'undefined') {
      fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:useEffect expenses',message:'expenses effect completed',data:{totalEffectTime:effectEndTime-effectStartTime},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'G'})}).catch(()=>{});
    }
    // #endregion
  }, [expenses, sortRows]);

  // Re-sort when sort preferences change
  useEffect(() => {
    if (rows.length > 0) {
      setRows(prevRows => sortRows(prevRows));
    }
  }, [sortColumn, sortDirection, sortRows]);

  // Find similar transactions - returns two arrays: vendor-only matches and vendor+description matches
  // Uses normalized vendor matching to find expenses with the same vendor (and optionally description)
  const findSimilarTransactions = useCallback((expense: Expense, allExpenses: Expense[]): {
    vendorOnlyMatches: Expense[];
    vendorAndDescMatches: Expense[];
  } => {
    const normalizeText = (str: string | null) => (str ?? '').trim().toLowerCase();
    const vendorKey = (e: Expense) => normalizeVendor(e.vendor || extractVendor(e.description) || null);

    const expenseVendor = vendorKey(expense);
    if (!expenseVendor) return { vendorOnlyMatches: [], vendorAndDescMatches: [] };

    const expenseDesc = normalizeText(expense.description);

    const vendorOnlyMatches: Expense[] = [];
    const vendorAndDescMatches: Expense[] = [];

    for (const e of allExpenses) {
      if (e.id === expense.id) continue; // Exclude the current expense
      
      const eVendor = vendorKey(e);
      if (eVendor !== expenseVendor) continue; // Must match vendor
      
      const eDesc = normalizeText(e.description);
      if (eDesc === expenseDesc) {
        vendorAndDescMatches.push(e);
      } else {
        vendorOnlyMatches.push(e);
      }
    }

    return { vendorOnlyMatches, vendorAndDescMatches };
  }, []);

  // Create or update transaction rule
  const createOrUpdateTransactionRule = useCallback(async (
    vendor: string | null,
    description: string | null,
    categoryId: string,
    householdId: string | null
  ): Promise<void> => {
    if (!householdId) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      // Check for existing rule with same pattern
      const { data: existingRules, error: fetchError } = await supabase
        .from('transaction_rules')
        .select('*')
        .eq('household_id', householdId)
        .eq('category_id', categoryId)
        .eq('match_type', 'combination');

      if (fetchError) throw fetchError;

      // Find matching rule
      const matchingRule = existingRules?.find((rule) => {
        const pattern = rule.match_pattern as any;
        // Handle both empty string and null/undefined as equivalent
        const ruleVendor = pattern.vendor ?? '';
        const ruleDesc = pattern.description ?? '';
        const normalize = (str: string | null) => (str ?? '').trim().toLowerCase();
        const normalizeV = (v: unknown) => normalizeVendor(typeof v === 'string' ? v : null);
        const normalizedVendor = normalizeVendor(vendor || extractVendor(description) || null);
        return (
          normalizeV(ruleVendor) === normalizedVendor &&
          normalize(ruleDesc) === normalize(description)
        );
      });

      if (matchingRule) {
        // Update existing rule to ensure it's active
        const { error: updateError } = await supabase
          .from('transaction_rules')
          .update({ is_active: true, priority: 100 })
          .eq('id', matchingRule.id);

        if (updateError) throw updateError;
      } else {
        // Create new rule
        const ruleName = description
          ? `Auto: ${vendor || extractVendor(description) || 'Vendor'} - ${description}`
          : 'Auto-categorization rule';

        const normalizedVendor = normalizeVendor(vendor || extractVendor(description) || null);

        const matchPattern: any = {
          combination: 'AND' as const,
          vendor: normalizedVendor,
          description: description ?? '',
        };

        // If we couldn't determine a vendor, don't include vendor in the rule (avoid overly broad vendor matching).
        if (!normalizedVendor) {
          delete matchPattern.vendor;
        }

        const { error: insertError } = await supabase.from('transaction_rules').insert({
          household_id: householdId,
          user_id: user.id,
          name: ruleName,
          priority: 100,
          match_type: 'combination',
          match_pattern: matchPattern,
          category_id: categoryId,
          is_active: true,
        });

        if (insertError) throw insertError;
      }
    } catch (error) {
      console.error('Error creating/updating transaction rule:', error);
      throw error;
    }
  }, [supabase]);

  // Optimized filtering - pre-compute searchable strings once
  const rowSearchStrings = useMemo(() => {
    return rows.map((r) => {
      const categoryName = categoryMap.get(r.category_id) || '';
      const memberName = memberMap.get(r.payer_id) || '';
      const accountName = r.account_id ? (accountMap.get(r.account_id) || '') : '';
      
      return [
        r.date,
        formatDate(r.date),
        String(r.amount),
        categoryName,
        r.vendor ?? '',
        r.description ?? '',
        memberName,
        accountName,
      ].join(' ').toLowerCase();
    });
  }, [rows, categoryMap, memberMap, accountMap]);

  const visibleRowIndexes = useMemo(() => {
    const q = debouncedFilterValue.trim().toLowerCase();
    if (q.length === 0) return rows.map((_, idx) => idx);

    const out: number[] = [];
    for (let i = 0; i < rowSearchStrings.length; i++) {
      if (rowSearchStrings[i].includes(q)) {
        out.push(i);
      }
    }
    return out;
  }, [debouncedFilterValue, rowSearchStrings]);

  const visibleRowCount = visibleRowIndexes.length;

  // Calculate grid height for virtual scrolling - use fixed viewport-based height
  // This ensures the grid uses virtual scrolling and doesn't render all rows at once
  // The DataEditor component handles virtual scrolling internally when height is fixed
  const rowHeight = 36;
  const headerHeight = 36;
  const dataEditorHeight = useMemo(() => {
    if (typeof window === 'undefined') return 600;
    const viewportHeight = window.innerHeight;
    
    // If in fixed mode, calculate from sticky header to bottom of viewport
    // This maximizes available space for the grid when it's fixed to the viewport
    if (isFixedMode) {
      // Get the actual header height from MainLayout's padding
      const scrollContainer = document.querySelector('[class*="overflow-y-auto"]');
      let stickyHeaderHeight = 80; // Default fallback
      if (scrollContainer) {
        const style = window.getComputedStyle(scrollContainer);
        const paddingTop = parseInt(style.paddingTop, 10);
        if (paddingTop > 0) {
          stickyHeaderHeight = paddingTop;
        }
      }
      
      // Toolbar height is approximately 40-50px (h-8 buttons + padding)
      const toolbarHeight = 50;
      // Small padding for visual spacing
      const padding = 20;
      
      // Calculate height from header to bottom of viewport
      return viewportHeight - stickyHeaderHeight - toolbarHeight - padding;
    }
    
    // Original logic for non-fixed mode (static positioning)
    // Use a fixed height based on viewport (about 70% of viewport) to enable virtual scrolling
    // This prevents rendering all rows at once, improving performance for large datasets
    // Reserve space for header, toolbar, and padding (~200px)
    const reservedSpace = 200;
    const maxHeight = Math.floor((viewportHeight - reservedSpace) * 0.7);
    // Ensure minimum height for usability, but cap at reasonable maximum
    return Math.max(460, Math.min(maxHeight, 800));
  }, [isFixedMode]);

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
          parentColor: groupBaseColor ?? undefined,
          icon: cat.icon,
        });
      });
    }

    // Uncategorized -> show under "Other" group
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

  // Compute the update data for a cell edit - converts grid cell value to expense update
  // Returns both the update data and the next expense state for optimistic UI updates
  const computeUpdateForCell = useCallback(
    (col: number, expense: Expense, newValue: EditableGridCell) => {
      let updateData: Partial<Expense> | null = null;
      let nextExpense: Expense | null = null;
      const getText = () => String((newValue as any).data ?? '').trim();

      switch (col) {
        case 0: {
          // Date column
          if (newValue.kind !== GridCellKind.Text) return null;
          const nextDate = getText();
          if (!nextDate) return null;
          updateData = { date: nextDate };
          nextExpense = { ...expense, date: nextDate };
          break;
        }
        case 1: {
          // Amount column
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
          // Category column
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
          // Vendor column
          if (newValue.kind !== GridCellKind.Text) return null;
          const input = String((newValue as any).data ?? '');
          const nextVendor = input.trim() === '' ? null : input;
          updateData = { vendor: nextVendor };
          nextExpense = { ...expense, vendor: nextVendor };
          break;
        }
        case 4: {
          // Description column
          if (newValue.kind !== GridCellKind.Text) return null;
          const input = String((newValue as any).data ?? '');
          const nextDesc = input.trim() === '' ? null : input;
          updateData = { description: nextDesc };
          nextExpense = { ...expense, description: nextDesc };
          break;
        }
        case 5: {
          // Payer column
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
        case 6: {
          // Account column
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

  // Handle cell edits - processes edits from the grid and applies them optimistically
  // IMPORTANT: This callback runs on the hot path of closing the overlay editor.
  // To maintain responsiveness, we:
  // 1. Collect patches without copying the entire rows array (O(n) operation)
  // 2. Return immediately to unblock the UI
  // 3. Apply updates in a microtask (scheduleMicrotask) after the editor closes
  // 4. Check for similar transactions when category changes to show bulk update dialog
  // 5. Track changes in undo history for Ctrl+Z support
  const handleCellsEdited = useCallback(
    (items: readonly EditListItem[]) => {
      if (items.length === 0) return true;
      const rowPatches: Array<{ realIdx: number; nextExpense: Expense }> = [];
      const updatesById = new Map<string, Partial<Expense>>();
      let categoryChange: { expense: Expense; categoryId: string } | null = null;

      for (const item of items) {
        const [col, rowIdx] = item.location;
        const realIdx = visibleRowIndexes[rowIdx];
        if (realIdx == null) continue;
        const expense = rows[realIdx];
        if (!expense) continue;

        const computed = computeUpdateForCell(col, expense, item.value);
        if (!computed) continue;

        // Check if this is a category change (column 2)
        if (col === 2 && computed.updateData.category_id) {
          categoryChange = {
            expense: computed.nextExpense,
            categoryId: computed.updateData.category_id,
          };
        }

        rowPatches.push({ realIdx, nextExpense: computed.nextExpense });

        const prev = updatesById.get(expense.id) ?? {};
        updatesById.set(expense.id, { ...prev, ...computed.updateData });
      }

      if (updatesById.size === 0) return true;

      // If category changed, check for similar transactions
      if (categoryChange) {
        const { vendorOnlyMatches: vendorOnly, vendorAndDescMatches: vendorAndDesc } = findSimilarTransactions(categoryChange.expense, rows);
        if (vendorOnly.length > 0 || vendorAndDesc.length > 0) {
          // Track this expense as locally updated
          locallyUpdatedExpenseIds.current.add(categoryChange.expense.id);
          
          // Store pending update and show dialog
          setPendingCategoryUpdate({
            expenseId: categoryChange.expense.id,
            categoryId: categoryChange.categoryId,
          });
          setPendingRuleCreation({
            vendor: categoryChange.expense.vendor || extractVendor(categoryChange.expense.description),
            description: categoryChange.expense.description,
            categoryId: categoryChange.categoryId,
          });
          setVendorOnlyMatches(vendorOnly);
          setVendorAndDescMatches(vendorAndDesc);
          setShowSimilarTransactionsDialog(true);
          
          // Apply local UI update for the edited expense only
          scheduleMicrotask(() => {
            setRows((prevRows) => {
              const next = [...prevRows];
              // Find the patch for the category change expense by matching ID
              const patch = rowPatches.find((p) => p.nextExpense.id === categoryChange!.expense.id);
              if (patch && patch.realIdx >= 0 && patch.realIdx < next.length) {
                next[patch.realIdx] = patch.nextExpense;
              }
              return next;
            });
          });
          
          return true; // Don't persist yet, wait for user confirmation
        }
      }

      scheduleMicrotask(() => {
        // Store previous state for undo before applying changes
        const previousStates: Array<{ expenseId: string; previousExpense: Expense }> = [];
        for (const p of rowPatches) {
          const originalExpense = rows[p.realIdx];
          if (originalExpense && !locallyUpdatedExpenseIds.current.has(originalExpense.id)) {
            // Only track if this expense hasn't been persisted yet
            previousStates.push({ expenseId: originalExpense.id, previousExpense: originalExpense });
          }
        }

        // Add to undo history (limit size)
        if (previousStates.length > 0) {
          setUndoHistory((prev) => {
            const combined = [...prev, ...previousStates];
            return combined.slice(-MAX_HISTORY_SIZE);
          });
          // Clear redo history when new changes are made
          setRedoHistory([]);
        }

        // Apply local UI updates
        setRows((prevRows) => {
          if (rowPatches.length === 0) return prevRows;
          const next = [...prevRows];
          for (const p of rowPatches) {
            if (p.realIdx >= 0 && p.realIdx < next.length) {
              next[p.realIdx] = p.nextExpense;
            }
          }
          return next;
        });

        // Persist updates (async; do not block UI)
        const updates = Array.from(updatesById.entries()).map(([id, data]) => ({ id, data }));
        if (onBulkUpdate) {
          try {
            void Promise.resolve(onBulkUpdate(updates)).then(() => {
              // Clear undo history for persisted changes
              setUndoHistory((prev) => prev.filter((h) => !updates.some((u) => u.id === h.expenseId)));
            }).catch(() => {});
          } catch {
            // ignore
          }
        } else {
          for (const u of updates) {
            try {
              void Promise.resolve(onUpdate(u.id, u.data)).then(() => {
                // Clear undo history for persisted changes
                setUndoHistory((prev) => prev.filter((h) => h.expenseId !== u.id));
              }).catch(() => {});
            } catch {
              // ignore
            }
          }
        }
      });

      return true;
    },
    [computeUpdateForCell, onBulkUpdate, onUpdate, rows, visibleRowIndexes, findSimilarTransactions]
  );

  // (Copy button removed per request)

  // Header menu state
  const [headerMenu, setHeaderMenu] = useState<{
    col: number;
    bounds: Rectangle;
  } | null>(null);

  const glideColumns = useMemo<GridColumn[]>(() => {
    return [
      { 
        id: 'date', 
        title: 'Date',
        width: 120,
        hasMenu: true,
      },
      { 
        id: 'amount', 
        title: 'Amount',
        width: 120,
        hasMenu: true,
      },
      { 
        id: 'category', 
        title: 'Category',
        width: 180,
        hasMenu: true,
      },
      { 
        id: 'vendor', 
        title: 'Vendor',
        width: 200,
        hasMenu: true,
      },
      { 
        id: 'description', 
        title: 'Description',
        width: 260, 
        grow: 1,
        hasMenu: true,
      },
      { 
        id: 'payer', 
        title: 'Partner',
        width: 180,
        hasMenu: true,
      },
      { 
        id: 'account', 
        title: 'Account',
        width: 220,
        hasMenu: true,
      },
    ];
  }, []);

  // Handle header menu click
  const handleHeaderMenuClick = useCallback((col: number, bounds: Rectangle) => {
    setHeaderMenu({ col, bounds });
  }, []);

  // Handle sort selection from menu
  const handleSort = useCallback((direction: 'asc' | 'desc') => {
    if (!headerMenu) return;
    const column = glideColumns[headerMenu.col];
    if (!column) return;
    
    setSortColumn(column.id);
    setSortDirection(direction);
    setHeaderMenu(null);
  }, [headerMenu, glideColumns]);

  const handleViewSpending = useCallback((categoryIds: string[]) => {
    if (categoryIds.length === 0) return;
    // For multiple categories, pass them as comma-separated
    const categoryParam = categoryIds.join(',');
    router.push(`/analytics?category=${categoryParam}`);
  }, [router]);

  // Undo handler
  const handleUndo = useCallback(() => {
    if (undoHistory.length === 0) return;

    const lastChange = undoHistory[undoHistory.length - 1];
    const expenseId = lastChange.expenseId;
    const previousExpense = lastChange.previousExpense;

    // Find current state of this expense
    const currentExpense = rows.find((e) => e.id === expenseId);
    if (!currentExpense) return;

    // Store current state in redo history (this is the state we're reverting FROM, which we'll reapply on redo)
    setRedoHistory((prev) => [...prev, { expenseId, nextExpense: currentExpense }]);

    // Revert to previous state
    setRows((prevRows) => {
      return prevRows.map((expense) => {
        if (expense.id === expenseId) {
          return previousExpense;
        }
        return expense;
      });
    });

    // Remove from undo history
    setUndoHistory((prev) => prev.slice(0, -1));

    // Remove from locally updated tracking if it was there
    locallyUpdatedExpenseIds.current.delete(expenseId);
  }, [undoHistory, rows]);

  // Redo handler
  const handleRedo = useCallback(() => {
    if (redoHistory.length === 0) return;

    const lastChange = redoHistory[redoHistory.length - 1];
    const expenseId = lastChange.expenseId;
    const nextExpense = lastChange.nextExpense;

    // Find current state of this expense
    const currentExpense = rows.find((e) => e.id === expenseId);
    if (!currentExpense) return;

    // Store current state in undo history
    setUndoHistory((prev) => [...prev, { expenseId, previousExpense: currentExpense }]);

    // Reapply the change
    setRows((prevRows) => {
      return prevRows.map((expense) => {
        if (expense.id === expenseId) {
          return nextExpense;
        }
        return expense;
      });
    });

    // Remove from redo history
    setRedoHistory((prev) => prev.slice(0, -1));

    // Add back to locally updated tracking
    locallyUpdatedExpenseIds.current.add(expenseId);
  }, [redoHistory, rows]);

  // Performance optimization: Memoize formatted dates and amounts to avoid re-formatting on every render
  // These Maps cache formatted strings by their raw values, so we only format each unique date/amount once
  const formattedDates = useMemo(() => {
    const map = new Map<string, string>();
    rows.forEach((r) => {
      if (!map.has(r.date)) {
        map.set(r.date, formatDate(r.date));
      }
    });
    return map;
  }, [rows]);

  const formattedAmounts = useMemo(() => {
    const map = new Map<number, string>();
    rows.forEach((r) => {
      if (!map.has(r.amount)) {
        map.set(r.amount, formatCurrency(r.amount));
      }
    });
    return map;
  }, [rows]);

  // Get theme override for a row based on payer's color
  const getRowThemeOverride = useCallback((payerId: string): Partial<GlideTheme> | undefined => {
    const userColor = userColors[payerId];
    if (!userColor) return undefined;
    
    // Use subtle background color with alpha for row highlighting
    return {
      bgCell: withAlpha(userColor, appTheme === 'dark' ? 0.15 : 0.08),
      bgCellMedium: withAlpha(userColor, appTheme === 'dark' ? 0.12 : 0.06),
    };
  }, [userColors, appTheme]);

  const getCellContent = useCallback((cell: Item): GridCell => {
    const [col, rowIdx] = cell;
    const realIdx = visibleRowIndexes[rowIdx];
    const row = realIdx == null ? undefined : rows[realIdx];

    if (!row) {
      return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
    }

    // Get theme override for this row based on payer color
    const rowThemeOverride = getRowThemeOverride(row.payer_id);

    switch (col) {
      case 0:
        // Date column - use memoized format
        return {
          kind: GridCellKind.Text,
          data: row.date,
          displayData: formattedDates.get(row.date) || formatDate(row.date),
          allowOverlay: true,
          twocentsEditor: 'date',
          themeOverride: rowThemeOverride,
        } as TwocentsGridCell;
      case 1:
        // Amount column - use memoized format
        return {
          kind: GridCellKind.Number,
          data: row.amount,
          displayData: formattedAmounts.get(row.amount) || formatCurrency(row.amount),
          allowOverlay: true,
          themeOverride: rowThemeOverride,
        };
      case 2: {
        // Category column - use map lookup
        const name = categoryMap.get(row.category_id) || '';
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
        // Vendor column
        return {
          kind: GridCellKind.Text,
          data: row.vendor ?? '',
          displayData: row.vendor ?? '',
          allowOverlay: true,
          themeOverride: rowThemeOverride,
        };
      case 4:
        // Description column
        return {
          kind: GridCellKind.Text,
          data: row.description ?? '',
          displayData: row.description ?? '',
          allowOverlay: true,
          themeOverride: rowThemeOverride,
        };
      case 5: {
        // Payer column - use map lookup
        const name = memberMap.get(row.payer_id) || '';
        return {
          kind: GridCellKind.Text,
          data: row.payer_id,
          displayData: name,
          allowOverlay: true,
          twocentsEditor: 'payer',
          themeOverride: rowThemeOverride,
        } as TwocentsGridCell;
      }
      case 6: {
        // Account column - use map lookup
        const name = row.account_id ? (accountMap.get(row.account_id) || '') : '';
        return {
          kind: GridCellKind.Text,
          data: row.account_id ?? '',
          displayData: name,
          allowOverlay: true,
          twocentsEditor: 'account',
          themeOverride: rowThemeOverride,
        } as TwocentsGridCell;
      }
      default:
        return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
    }
  }, [rows, categoryMap, memberMap, accountMap, visibleRowIndexes, formattedDates, formattedAmounts, getRowThemeOverride]);

  // Performance optimization: Lazy selection computation
  // Only compute selection data (expense IDs, category IDs) when buttons are clicked, not during selection
  // This prevents lag during multiselect operations by deferring expensive computations
  const getSelectionData = useCallback(() => {
    if (!gridSelection.rows || gridSelection.rows.length === 0) {
      return {
        expenseIds: [] as string[],
        categoryIds: [] as string[],
      };
    }

    const rowIndices = Array.from(gridSelection.rows);
    const expenseIds: string[] = [];
    const categoryIdsSet = new Set<string>();

    // Single pass through selected rows
    for (let i = 0; i < rowIndices.length; i++) {
      const rowIdx = rowIndices[i];
      const realIdx = visibleRowIndexes[rowIdx];
      if (realIdx != null) {
        const row = rows[realIdx];
        if (row) {
          expenseIds.push(row.id);
          categoryIdsSet.add(row.category_id);
        }
      }
    }

    return {
      expenseIds,
      categoryIds: Array.from(categoryIdsSet),
    };
  }, [gridSelection.rows, rows, visibleRowIndexes]);

  // Only compute selection count for button visibility (lightweight)
  const selectedRowCount = useMemo(() => {
    return gridSelection.rows?.length ?? 0;
  }, [gridSelection.rows]);

  const handleFlagSelected = useCallback(async () => {
    // Compute selection data only when button is clicked
    const selectionData = getSelectionData();
    if (selectionData.expenseIds.length === 0 || !householdId) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      // Check if all selected are flagged
      const flaggedIds = new Set(
        Object.keys(expenseFlags).filter(id => expenseFlags[id])
      );
      const allSelectedFlagged = selectionData.expenseIds.every(id => flaggedIds.has(id));

      if (allSelectedFlagged) {
        // Unflag all selected - no dialog needed for unflagging
        const unflagPromises = selectionData.expenseIds.map(async (expenseId: string) => {
          // Resolve all flags for this expense
          const { error: flagError } = await supabase
            .from('expense_flags')
            .update({ resolved_at: new Date().toISOString() })
            .eq('expense_id', expenseId)
            .is('resolved_at', null);

          if (flagError) throw flagError;

          // Update expense status
          const { error: statusError } = await supabase
            .from('expenses')
            .update({ status: 'approved' })
            .eq('id', expenseId);

          if (statusError) throw statusError;
        });

        await Promise.all(unflagPromises);
        onFlagChange?.();
      } else {
        // Flag all selected - show dialog first to get comment
        // Filter out already flagged expenses
        const expensesToFlag = selectionData.expenseIds.filter(
          (expenseId) => !expenseFlags[expenseId]
        );
        
        if (expensesToFlag.length === 0) return;
        
        // Store expense IDs and show dialog
        setPendingFlagExpenseIds(expensesToFlag);
        setShowFlagCommentDialog(true);
      }
    } catch (error) {
      console.error('Error flagging expenses:', error);
    }
  }, [getSelectionData, householdId, supabase, expenseFlags, onFlagChange]);

  const handleConfirmFlag = useCallback(async (comment: string) => {
    if (pendingFlagExpenseIds.length === 0 || !householdId) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const finalComment = comment || 'Flagged for review';

      // Flag all selected - only flag expenses that aren't already flagged
      const flagPromises = pendingFlagExpenseIds.map(async (expenseId) => {
        // Create flag with comment
        const { error: flagError } = await supabase.from('expense_flags').insert({
          expense_id: expenseId,
          user_id: user.id,
          flag_type: 'review',
          notes: finalComment,
        });

        if (flagError) throw flagError;

        // Also create a comment entry so it appears in the chat dialog
        const { error: commentError } = await supabase.from('expense_comments').insert({
          expense_id: expenseId,
          user_id: user.id,
          comment: finalComment,
        });

        if (commentError) throw commentError;

        // Update expense status
        const { error: statusError } = await supabase
          .from('expenses')
          .update({ status: 'flagged' })
          .eq('id', expenseId);

        if (statusError) throw statusError;
      });

      await Promise.all(flagPromises);

      // Call onFlagChange to refresh flags
      onFlagChange?.();
      
      // Notify ReviewGrid to refresh after a delay
      onTransactionsFlagged?.();
      
      // Clear pending state
      setPendingFlagExpenseIds([]);
    } catch (error) {
      console.error('Error flagging expenses:', error);
      setPendingFlagExpenseIds([]);
    }
  }, [pendingFlagExpenseIds, householdId, supabase, onFlagChange, onTransactionsFlagged]);

  const handleCancelFlag = useCallback(() => {
    setPendingFlagExpenseIds([]);
  }, []);

  const handleDeleteSelected = useCallback(async () => {
    if (!onBulkDelete) return;
    
    // Compute selection data only when button is clicked
    const selectionData = getSelectionData();
    if (selectionData.expenseIds.length === 0) return;
    
    // #region agent log
    fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:handleDeleteSelected',message:'handleDeleteSelected entry',data:{expenseIdsCount:selectionData.expenseIds.length},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'D'})}).catch(()=>{});
    // #endregion
    
    setIsDeleting(true);
    try {
      await onBulkDelete(selectionData.expenseIds);
      // #region agent log
      fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:handleDeleteSelected',message:'handleDeleteSelected success',data:{},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'D'})}).catch(()=>{});
      // #endregion
      setShowDeleteConfirm(false);
      // Clear selection after deletion
      setGridSelection({
        columns: CompactSelection.empty(),
        rows: CompactSelection.empty(),
      });
    } catch (error: any) {
      // #region agent log
      fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'ExpensesSpreadsheet.tsx:handleDeleteSelected',message:'handleDeleteSelected error',data:{errorMessage:error?.message,errorStack:error?.stack},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'D'})}).catch(()=>{});
      // #endregion
      console.error('Error deleting expenses:', error);
      // Show user-friendly error message
      alert(`Failed to delete expenses: ${error?.message || 'Unknown error'}`);
    } finally {
      setIsDeleting(false);
    }
  }, [onBulkDelete, getSelectionData]);

  // Handler for applying category to all similar transactions (from a specific grid)
  const handleApplyToAll = useCallback(async (transactions: Expense[]) => {
    if (!pendingCategoryUpdate || transactions.length === 0) return;

    const updates = [
      { id: pendingCategoryUpdate.expenseId, data: { category_id: pendingCategoryUpdate.categoryId } },
      ...transactions.map((exp) => ({
        id: exp.id,
        data: { category_id: pendingCategoryUpdate.categoryId },
      })),
    ];

    const updatedExpenseIds = new Set(updates.map(u => u.id));

    try {
      // Track these expense IDs as locally updated
      updatedExpenseIds.forEach(id => locallyUpdatedExpenseIds.current.add(id));

      // Update local state immediately to reflect changes
      setRows((prevRows) => {
        return prevRows.map((expense) => {
          if (updatedExpenseIds.has(expense.id)) {
            return { ...expense, category_id: pendingCategoryUpdate.categoryId };
          }
          return expense;
        });
      });

      // Update the relevant state arrays to reflect changes
      setVendorOnlyMatches((prev) => {
        return prev.map((exp) => {
          if (updatedExpenseIds.has(exp.id)) {
            return { ...exp, category_id: pendingCategoryUpdate.categoryId };
          }
          return exp;
        });
      });
      setVendorAndDescMatches((prev) => {
        return prev.map((exp) => {
          if (updatedExpenseIds.has(exp.id)) {
            return { ...exp, category_id: pendingCategoryUpdate.categoryId };
          }
          return exp;
        });
      });

      if (onBulkUpdate) {
        await onBulkUpdate(updates);
      } else {
        for (const update of updates) {
          await onUpdate(update.id, update.data);
        }
      }

      // Close dialog and clear state
      setShowSimilarTransactionsDialog(false);
      setPendingCategoryUpdate(null);
      setPendingRuleCreation(null);
      setVendorOnlyMatches([]);
      setVendorAndDescMatches([]);
      
      // Clear tracking after a short delay to allow parent to refresh
      // The useEffect will automatically clear them when props are updated
      setTimeout(() => {
        updatedExpenseIds.forEach(id => locallyUpdatedExpenseIds.current.delete(id));
      }, 2000);
    } catch (error) {
      console.error('Error applying category to all:', error);
      // On error, revert local state changes and clear tracking
      updatedExpenseIds.forEach(id => locallyUpdatedExpenseIds.current.delete(id));
      setRows((prevRows) => {
        // Find original expenses from props to revert
        const expenseMap = new Map(expenses.map(e => [e.id, e]));
        return prevRows.map((expense) => {
          const original = expenseMap.get(expense.id);
          return original || expense;
        });
      });
    }
  }, [pendingCategoryUpdate, onBulkUpdate, onUpdate, expenses]);

  // Handler for applying category to all and creating rule (from a specific grid)
  const handleApplyToAllAndCreateRule = useCallback(async (transactions: Expense[]) => {
    if (!pendingCategoryUpdate || !pendingRuleCreation || !householdId || transactions.length === 0) return;

    try {
      // Create or update the transaction rule
      await createOrUpdateTransactionRule(
        pendingRuleCreation.vendor,
        pendingRuleCreation.description,
        pendingRuleCreation.categoryId,
        householdId
      );

      // Apply category to all similar transactions
      await handleApplyToAll(transactions);
    } catch (error) {
      console.error('Error applying category and creating rule:', error);
    }
  }, [pendingCategoryUpdate, pendingRuleCreation, householdId, createOrUpdateTransactionRule, handleApplyToAll]);

  // Handler for applying category only to current transaction
  const handleApplyToCurrentOnly = useCallback(async () => {
    if (!pendingCategoryUpdate) return;

    try {
      const update = {
        id: pendingCategoryUpdate.expenseId,
        data: { category_id: pendingCategoryUpdate.categoryId },
      };

      // Track this expense as locally updated
      locallyUpdatedExpenseIds.current.add(pendingCategoryUpdate.expenseId);

      // Update local state immediately
      setRows((prevRows) => {
        return prevRows.map((expense) => {
          if (expense.id === pendingCategoryUpdate.expenseId) {
            return { ...expense, category_id: pendingCategoryUpdate.categoryId };
          }
          return expense;
        });
      });

      // Persist the update
      if (onBulkUpdate) {
        await onBulkUpdate([update]);
      } else {
        await onUpdate(update.id, update.data);
      }

      // Close dialog and clear state
      setShowSimilarTransactionsDialog(false);
      setPendingCategoryUpdate(null);
      setPendingRuleCreation(null);
      setVendorOnlyMatches([]);
      setVendorAndDescMatches([]);

      // Clear tracking after a short delay
      setTimeout(() => {
        locallyUpdatedExpenseIds.current.delete(pendingCategoryUpdate.expenseId);
      }, 2000);
    } catch (error) {
      console.error('Error applying category to current transaction:', error);
      // On error, revert local state changes and clear tracking
      if (pendingCategoryUpdate) {
        locallyUpdatedExpenseIds.current.delete(pendingCategoryUpdate.expenseId);
        setRows((prevRows) => {
          const expenseMap = new Map(expenses.map(e => [e.id, e]));
          return prevRows.map((expense) => {
            const original = expenseMap.get(expense.id);
            return original || expense;
          });
        });
      }
    }
  }, [pendingCategoryUpdate, onBulkUpdate, onUpdate, expenses]);

  // Handler for canceling the dialog
  const handleCancelSimilarDialog = useCallback(() => {
    // Revert the local UI change for the edited expense
    if (pendingCategoryUpdate) {
      // Remove from tracking since we're reverting
      locallyUpdatedExpenseIds.current.delete(pendingCategoryUpdate.expenseId);
      
      setRows((prevRows) => {
        const next = [...prevRows];
        const expenseIdx = next.findIndex((e) => e.id === pendingCategoryUpdate.expenseId);
        if (expenseIdx >= 0) {
          // Find original expense from props
          const originalExpense = expenses.find((e) => e.id === pendingCategoryUpdate.expenseId);
          if (originalExpense) {
            next[expenseIdx] = originalExpense;
          }
        }
        return next;
      });
    }

    // Close dialog and clear state
    setShowSimilarTransactionsDialog(false);
    setPendingCategoryUpdate(null);
    setPendingRuleCreation(null);
    setVendorOnlyMatches([]);
    setVendorAndDescMatches([]);
  }, [pendingCategoryUpdate, expenses]);

  // Unified cell content function for similar transactions grids (vendor-only and vendor+description)
  // Accepts the expense array as parameter to avoid code duplication
  const getSimilarTransactionsCellContent = useCallback((expenseArray: Expense[]) => {
    return (cell: Item): GridCell => {
      const [col, rowIdx] = cell;
      const expense = expenseArray[rowIdx];

      if (!expense) {
        return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
      }

      switch (col) {
        case 0:
          // Date column
          return {
            kind: GridCellKind.Text,
            data: expense.date,
            displayData: formatDate(expense.date),
            allowOverlay: false,
          };
        case 1:
          // Amount column
          return {
            kind: GridCellKind.Number,
            data: expense.amount,
            displayData: formatCurrency(expense.amount),
            allowOverlay: false,
          };
        case 2: {
          // Category column - editable
          const name = categoryMap.get(expense.category_id) || '';
          return {
            kind: GridCellKind.Text,
            data: expense.category_id,
            displayData: name,
            allowOverlay: true,
            twocentsEditor: 'category',
          } as TwocentsGridCell;
        }
        case 3:
          // Vendor column
          return {
            kind: GridCellKind.Text,
            data: expense.vendor ?? '',
            displayData: expense.vendor ?? '',
            allowOverlay: false,
          };
        case 4:
          // Description column
          return {
            kind: GridCellKind.Text,
            data: expense.description ?? '',
            displayData: expense.description ?? '',
            allowOverlay: false,
          };
        default:
          return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
      }
    };
  }, [categoryMap]);

  // Unified cell edit handler for similar transactions grids
  // Accepts the expense array and state setter to avoid code duplication
  const createSimilarTransactionsCellsEditedHandler = useCallback(
    (expenseArray: Expense[], setExpenseArray: React.Dispatch<React.SetStateAction<Expense[]>>) => {
      return (items: readonly EditListItem[]) => {
        if (items.length === 0) return true;

        const updates: Array<{ id: string; data: Partial<Expense> }> = [];

        for (const item of items) {
          const [col, rowIdx] = item.location;
          const expense = expenseArray[rowIdx];
          if (!expense) continue;

          // Only handle category edits (column 2)
          if (col === 2 && item.value.kind === GridCellKind.Text) {
            const input = String((item.value as any).data ?? '').trim();
            if (!input) continue;
            const match =
              categories.find((c) => c.id === input) ??
              categories.find((c) => c.name.toLowerCase() === input.toLowerCase());
            if (!match) continue;

            updates.push({
              id: expense.id,
              data: { category_id: match.id },
            });
          }
        }

        if (updates.length === 0) return true;

        // Update local state
        setExpenseArray((prev) => {
          return prev.map((exp) => {
            const update = updates.find((u) => u.id === exp.id);
            return update ? { ...exp, category_id: update.data.category_id! } : exp;
          });
        });

        // Persist updates
        scheduleMicrotask(() => {
          if (onBulkUpdate) {
            try {
              void Promise.resolve(onBulkUpdate(updates)).catch(() => {});
            } catch {
              // ignore
            }
          } else {
            for (const u of updates) {
              try {
                void Promise.resolve(onUpdate(u.id, u.data)).catch(() => {});
              } catch {
                // ignore
              }
            }
          }
        });

        return true;
      };
    },
    [categories, onBulkUpdate, onUpdate]
  );

  // Columns for similar transactions dialog grid
  const similarTransactionsColumns = useMemo<GridColumn[]>(() => {
    return [
      { id: 'date', title: 'Date', width: 120 },
      { id: 'amount', title: 'Amount', width: 120 },
      { id: 'category', title: 'Category', width: 180 },
      { id: 'vendor', title: 'Vendor', width: 200 },
      { id: 'description', title: 'Description', width: 260, grow: 1 },
    ];
  }, []);

  // Calculate responsive height for similar transactions grids
  // Accounts for dialog header, footer, buttons, and spacing
  const similarTransactionsGridHeight = useMemo(() => {
    if (typeof window === 'undefined') return 300;
    const viewportHeight = window.innerHeight;
    // Reserve space for: dialog header (~120px), footer (~60px), grid buttons (~40px), padding (~40px)
    const reservedSpace = 260;
    const availableHeight = viewportHeight * 0.9 - reservedSpace; // 90vh dialog max height
    // Use a reasonable max height (around 400px) but allow it to be smaller on smaller screens
    return Math.min(Math.max(availableHeight / 2, 250), 400);
  }, []);

  return (
    <div className="flex flex-col">
      {/* Toolbar */}
      {toolbarPortalEl && createPortal(
        <div className="flex items-center gap-2">
          <Input
            value={filterValue}
            onChange={(e) => setFilterValue(e.target.value)}
            placeholder="Filter…"
            className="h-8 w-44 text-xs"
          />
          <div className="text-xs text-muted-foreground">
            {visibleRowCount} / {rows.length} expense{rows.length !== 1 ? 's' : ''}
          </div>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleFlagSelected}
            disabled={selectedRowCount === 0}
            className="h-8 text-xs"
            title={selectedRowCount === 0 ? 'Select transactions to flag' : 'Flag selected transactions'}
          >
            <Flag className="mr-1 h-3 w-3" />
            Flag
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              const selectionData = getSelectionData();
              if (selectionData.categoryIds.length > 0) {
                handleViewSpending(selectionData.categoryIds);
              }
            }}
            disabled={selectedRowCount === 0}
            className="h-8 text-xs"
            title={selectedRowCount === 0 ? 'Select transactions to view spending' : 'View spending for selected categories'}
          >
            <BarChart3 className="mr-1 h-3 w-3" />
            View Spending
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setShowDeleteConfirm(true)}
            disabled={selectedRowCount === 0 || !onBulkDelete}
            className="h-8 text-xs text-destructive hover:text-destructive disabled:text-muted-foreground disabled:hover:text-muted-foreground"
            title={selectedRowCount === 0 ? 'Select transactions to delete' : 'Delete selected transactions'}
          >
            <Trash2 className="mr-1 h-3 w-3" />
            Delete
          </Button>
        </div>,
        toolbarPortalEl
      )}

      {/* Grid */}
      <div ref={gridRef} className="w-full overflow-hidden relative">
        <DataEditor
          columns={glideColumns}
          rows={visibleRowCount}
          getCellContent={getCellContent}
          onCellsEdited={handleCellsEdited}
          provideEditor={provideEditor}
          theme={glideTheme}
          showSearch={showSearch}
          onSearchClose={() => setShowSearch(false)}
          getCellsForSelection={true}
          gridSelection={gridSelection}
          onGridSelectionChange={setGridSelection}
          onHeaderMenuClick={handleHeaderMenuClick}
          rowMarkers="both"
          freezeColumns={2}
          width="100%"
          height={dataEditorHeight}
          rowHeight={rowHeight}
          headerHeight={headerHeight}
          fillHandle={true}
          allowedFillDirections="orthogonal"
          cellActivationBehavior="single-click"
          editOnType={true}
          onKeyDown={(e) => {
            const isSearch =
              (e.ctrlKey || e.metaKey) &&
              (e.key === 'f' || e.key === 'F' || e.rawEvent?.code === 'KeyF');
            if (isSearch) {
              setShowSearch(true);
              e.preventDefault();
              e.stopPropagation();
              e.cancel();
              return;
            }

            // Handle Ctrl-Z (undo) and Ctrl-Y (redo)
            const isUndo = (e.ctrlKey || e.metaKey) && (e.key === 'z' || e.key === 'Z') && !e.shiftKey;
            const isRedo = (e.ctrlKey || e.metaKey) && ((e.key === 'y' || e.key === 'Y') || (e.key === 'z' || e.key === 'Z') && e.shiftKey);
            
            if (isUndo) {
              e.preventDefault();
              e.stopPropagation();
              e.cancel();
              handleUndo();
              return;
            }

            if (isRedo) {
              e.preventDefault();
              e.stopPropagation();
              e.cancel();
              handleRedo();
              return;
            }
          }}
        />
      </div>

      {/* Delete Confirmation Dialog */}
      <Dialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Transactions</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete {selectedRowCount} transaction{selectedRowCount !== 1 ? 's' : ''}? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={() => setShowDeleteConfirm(false)}
              disabled={isDeleting}
            >
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={handleDeleteSelected}
              disabled={isDeleting || !onBulkDelete}
            >
              {isDeleting ? 'Deleting...' : 'Delete'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Similar Transactions Dialog */}
      <Dialog open={showSimilarTransactionsDialog} onOpenChange={handleCancelSimilarDialog}>
        <DialogContent className="max-w-7xl max-h-[90vh] flex flex-col">
          <DialogHeader>
            <DialogTitle>Similar Transactions Found</DialogTitle>
            <DialogDescription>
              Found {vendorOnlyMatches.length} transaction{vendorOnlyMatches.length !== 1 ? 's' : ''} with the same vendor (different descriptions) and {vendorAndDescMatches.length} transaction{vendorAndDescMatches.length !== 1 ? 's' : ''} with the same vendor and description. Apply the same category to all, or edit individual categories below.
            </DialogDescription>
          </DialogHeader>
          <div className="flex-1 overflow-hidden min-h-0">
            <div className="grid grid-cols-2 gap-4 h-full">
              {/* Vendor-only matches grid */}
              <div className="flex flex-col h-full">
                <div className="mb-2 text-sm font-medium flex-shrink-0">
                  Same Vendor Only ({vendorOnlyMatches.length})
                </div>
                <div className="flex-1 overflow-hidden min-h-0">
                  {vendorOnlyMatches.length > 0 ? (
                    <DataEditor
                      columns={similarTransactionsColumns}
                      rows={vendorOnlyMatches.length}
                      getCellContent={getSimilarTransactionsCellContent(vendorOnlyMatches)}
                      onCellsEdited={createSimilarTransactionsCellsEditedHandler(vendorOnlyMatches, setVendorOnlyMatches)}
                      provideEditor={provideEditor}
                      theme={glideTheme}
                      gridSelection={vendorOnlyGridSelection}
                      onGridSelectionChange={setVendorOnlyGridSelection}
                      rowMarkers="both"
                      width="100%"
                      height={similarTransactionsGridHeight}
                      rowHeight={rowHeight}
                      headerHeight={headerHeight}
                      cellActivationBehavior="single-click"
                      editOnType={true}
                    />
                  ) : (
                    <div className="flex items-center justify-center" style={{ height: similarTransactionsGridHeight }}>
                      <span className="text-sm text-muted-foreground">No transactions with same vendor only</span>
                    </div>
                  )}
                </div>
                {vendorOnlyMatches.length > 0 && (
                  <div className="flex gap-1.5 flex-shrink-0 mt-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleApplyToAll(vendorOnlyMatches)}
                      className="text-xs h-7 px-2"
                    >
                      Apply to All
                    </Button>
                    <Button
                      size="sm"
                      onClick={() => handleApplyToAllAndCreateRule(vendorOnlyMatches)}
                      disabled={!householdId}
                      className="text-xs h-7 px-2"
                    >
                      Apply & Create Rule
                    </Button>
                  </div>
                )}
              </div>
              
              {/* Vendor + description matches grid */}
              <div className="flex flex-col h-full">
                <div className="mb-2 text-sm font-medium flex-shrink-0">
                  Same Vendor + Description ({vendorAndDescMatches.length})
                </div>
                <div className="flex-1 overflow-hidden min-h-0">
                  {vendorAndDescMatches.length > 0 ? (
                    <DataEditor
                      columns={similarTransactionsColumns}
                      rows={vendorAndDescMatches.length}
                      getCellContent={getSimilarTransactionsCellContent(vendorAndDescMatches)}
                      onCellsEdited={createSimilarTransactionsCellsEditedHandler(vendorAndDescMatches, setVendorAndDescMatches)}
                      provideEditor={provideEditor}
                      theme={glideTheme}
                      gridSelection={vendorAndDescGridSelection}
                      onGridSelectionChange={setVendorAndDescGridSelection}
                      rowMarkers="both"
                      width="100%"
                      height={similarTransactionsGridHeight}
                      rowHeight={rowHeight}
                      headerHeight={headerHeight}
                      cellActivationBehavior="single-click"
                      editOnType={true}
                    />
                  ) : (
                    <div className="flex items-center justify-center" style={{ height: similarTransactionsGridHeight }}>
                      <span className="text-sm text-muted-foreground">No transactions with same vendor and description</span>
                    </div>
                  )}
                </div>
                {vendorAndDescMatches.length > 0 && (
                  <div className="flex gap-1.5 flex-shrink-0 mt-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleApplyToAll(vendorAndDescMatches)}
                      className="text-xs h-7 px-2"
                    >
                      Apply to All
                    </Button>
                    <Button
                      size="sm"
                      onClick={() => handleApplyToAllAndCreateRule(vendorAndDescMatches)}
                      disabled={!householdId}
                      className="text-xs h-7 px-2"
                    >
                      Apply & Create Rule
                    </Button>
                  </div>
                )}
              </div>
            </div>
          </div>
          <DialogFooter className="flex-shrink-0 gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCancelSimilarDialog}
              className="text-xs h-7 px-2"
            >
              Cancel
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleApplyToCurrentOnly}
              className="text-xs h-7 px-2"
            >
              Apply to This Transaction Only
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Header Menu */}
      {headerMenu && typeof window !== 'undefined' && createPortal(
        <div
          className="fixed z-50 min-w-[160px] rounded-md border border-border bg-popover p-1 shadow-md"
          style={{
            left: `${headerMenu.bounds.x}px`,
            top: `${headerMenu.bounds.y + headerMenu.bounds.height}px`,
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <button
            className="w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent hover:text-accent-foreground"
            onClick={() => handleSort('asc')}
          >
            Sort Ascending
          </button>
          <button
            className="w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent hover:text-accent-foreground"
            onClick={() => handleSort('desc')}
          >
            Sort Descending
          </button>
        </div>,
        document.body
      )}
      
      {/* Overlay to close menu on outside click */}
      {headerMenu && typeof window !== 'undefined' && createPortal(
        <div
          className="fixed inset-0 z-40"
          onClick={() => setHeaderMenu(null)}
        />,
        document.body
      )}

      <FlagCommentDialog
        open={showFlagCommentDialog}
        onOpenChange={setShowFlagCommentDialog}
        transactionCount={pendingFlagExpenseIds.length}
        onConfirm={handleConfirmFlag}
        onCancel={handleCancelFlag}
      />
    </div>
  );
}
