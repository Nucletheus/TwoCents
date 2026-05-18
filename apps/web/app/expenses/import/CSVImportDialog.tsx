'use client';

import { useState, useEffect, useRef, useCallback, useMemo, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import {
  DataEditor,
  GridCellKind,
  getDefaultTheme,
  withAlpha,
  type DrawCellCallback,
  type GridCell,
  type GridColumn,
  type Item,
  type EditListItem,
  type Theme as GlideTheme,
} from '@glideapps/glide-data-grid';
import '@glideapps/glide-data-grid/dist/index.css';
import { createClient } from '@/lib/supabase/client';
import type { Category, ColumnMapping, CSVImportPreset } from '@twocents/shared';
import {
  parseCSV,
  normalizeDate,
  normalizeAmount,
  autoDetectColumnMapping,
  matchFilenamePattern,
  extractVendor,
  normalizeVendor,
  processSavingsFromImport,
  getVendorCategories,
  checkForDuplicates,
  formatCurrency,
  formatDate,
} from '@twocents/shared';
import type { Expense } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import Button from '../../components/ui/Button';
import Input from '../../components/ui/Input';
import Select from '../../components/ui/Select';
import { useTheme as useAppTheme } from '../../components/ThemeProvider';
import { X, Upload, FileText, Wand2, Check } from 'lucide-react';

interface CSVImportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  householdId: string | null;
  categories: Category[];
  onImportComplete: (importedExpenses?: Array<{
    household_id: string;
    payer_id: string;
    amount: number;
    category_id?: string;
    description?: string;
    date: string;
    vendor?: string;
    status?: 'pending_review' | 'approved' | 'flagged';
  }>) => void;
}

interface EditableTransaction {
  id: string;
  date: string;
  amount: number;
  description: string;
  category_id: string;
  payer_id: string;
  account_id: string | null;
  vendor: string | null;
}

type AmountConflictField = 'amount' | 'expense' | 'credit';
type AmountConflict = {
  rowIdx: number;
  field: AmountConflictField;
  columns: string[];
  values: string[];
};

interface HouseholdMember {
  user_id: string;
  profiles: {
    name: string | null;
    email?: string | null;
  } | null;
}

interface Account {
  id: string;
  name: string;
  type: string;
  user_id?: string;
}

type CsvEditorKind = 'mapping';

type CsvGridCell = GridCell & {
  csvEditor?: CsvEditorKind;
};

type DropdownOption = {
  id: string;
  label: string;
  subLabel?: string;
  group?: string;
  color?: string;
  icon?: string | null;
};

function scheduleMicrotask(cb: () => void) {
  if (typeof queueMicrotask === 'function') queueMicrotask(cb);
  else Promise.resolve().then(cb);
}

function commitTextCell(
  value: any,
  nextData: string,
  nextDisplayData: string | undefined,
  onFinishedEditing: (newValue?: any, movement?: readonly [0 | 1 | -1, 0 | 1 | -1]) => void
) {
  onFinishedEditing(
    {
      ...value,
      kind: GridCellKind.Text,
      data: nextData,
      displayData: nextDisplayData ?? nextData,
      allowOverlay: true,
    },
    [0, 0]
  );
}

/**
 * Detect if a row looks like headers (contains header keywords) vs data
 * Returns true only if we can clearly identify header keywords.
 * Defaults to false (treat as data) if ambiguous to avoid missing the first transaction.
 */
function looksLikeHeaders(row: string[]): boolean {
  if (!row || row.length === 0) return false;
  
  const headerKeywords = [
    'date', 'amount', 'total', 'price', 'cost',
    'description', 'memo', 'note', 'details',
    'vendor', 'merchant', 'store',
    'category', 'type',
    'account', 'account name',
    'expense', 'credit', 'debit',
    'transaction', 'posted', 'balance'
  ];
  
  // Check each cell individually for header keywords
  let headerKeywordCount = 0;
  let dataLikeCount = 0;
  
  const datePattern = /^\d{1,2}[-\/]\d{1,2}[-\/]\d{2,4}$/;
  const amountPattern = /^-?\$?\d+\.?\d*$/;
  
  for (const cell of row) {
    const cellLower = cell.toLowerCase().trim();
    if (!cellLower) continue;
    
    // Check if cell contains header keywords
    let hasHeaderKeyword = false;
    for (const keyword of headerKeywords) {
      if (cellLower.includes(keyword)) {
        hasHeaderKeyword = true;
        headerKeywordCount++;
        break;
      }
    }
    
    // Check if cell looks like data (date or amount)
    if (!hasHeaderKeyword) {
      const cleaned = cell.replace(/[,\s]/g, '');
      if (datePattern.test(cell.trim()) || amountPattern.test(cleaned)) {
        dataLikeCount++;
      }
    }
  }
  
  // If we found header keywords in multiple cells, it's likely headers
  if (headerKeywordCount >= 2) {
    return true;
  }
  
  // If we found header keywords in one cell but also data-like cells, be conservative
  if (headerKeywordCount === 1 && dataLikeCount > 0) {
    return false;
  }
  
  // If we found header keywords in one cell and no data-like cells, it might be headers
  if (headerKeywordCount === 1 && dataLikeCount === 0) {
    return true;
  }
  
  // If no header keywords found, it's definitely data (not headers)
  // This ensures the first transaction isn't missed when there are no explicit headers
  return false;
}

function buildCsvColumns(fileHeaders: string[], fileRows: string[][]): GridColumn[] {
  const sample = fileRows.slice(0, 50);
  return fileHeaders.map((h, i) => {
    const header = (h ?? '').trim() || `Column ${i + 1}`;
    let maxLen = header.length;
    for (const r of sample) {
      const v = String(r?.[i] ?? '');
      if (v.length > maxLen) maxLen = v.length;
    }
    // Roughly 7px per character + padding. Clamp to keep reasonable.
    const width = Math.max(140, Math.min(420, Math.floor(maxLen * 7 + 48)));
    return { id: `csv-${i}`, title: header, width };
  });
}

function GridDropdownEditor({
  value,
  onFinishedEditing,
  initialValue,
  title,
  options,
  placeholder,
  footer,
}: {
  value: any;
  onFinishedEditing: (newValue?: any, movement?: readonly [0 | 1 | -1, 0 | 1 | -1]) => void;
  initialValue?: string;
  title: string;
  options: DropdownOption[];
  placeholder?: string;
  footer?: ReactNode;
}) {
  const [query, setQuery] = useState<string>(initialValue ?? '');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus({ preventScroll: true });
    if (query) {
      inputRef.current?.setSelectionRange(query.length, query.length);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return options;
    return options.filter((o) => {
      const hay = `${o.label} ${o.subLabel ?? ''} ${o.group ?? ''}`.toLowerCase();
      return hay.includes(q);
    });
  }, [options, query]);

  const grouped = useMemo(() => {
    const groups: Record<string, DropdownOption[]> = {};
    const ungrouped: DropdownOption[] = [];
    for (const opt of filtered) {
      if (opt.group) {
        if (!groups[opt.group]) groups[opt.group] = [];
        groups[opt.group].push(opt);
      } else {
        ungrouped.push(opt);
      }
    }
    const sortedGroups = Object.keys(groups).sort((a, b) => a.localeCompare(b));
    return { groups, sortedGroups, ungrouped };
  }, [filtered]);

  const flat = useMemo(() => {
    const out: DropdownOption[] = [];
    for (const g of grouped.sortedGroups) out.push(...grouped.groups[g]);
    out.push(...grouped.ungrouped);
    return out;
  }, [grouped]);

  const [highlighted, setHighlighted] = useState<number>(() => (flat.length > 0 ? 0 : -1));
  const listRef = useRef<HTMLDivElement>(null);
  const highlightedItemRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    setHighlighted(flat.length > 0 ? 0 : -1);
  }, [query, flat.length]);

  // Auto-scroll highlighted item into view
  useEffect(() => {
    if (highlighted >= 0 && highlightedItemRef.current) {
      highlightedItemRef.current.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest',
      });
    }
  }, [highlighted]);

  const commit = useCallback(
    (id: string) => {
      const opt = options.find((o) => o.id === id);
      commitTextCell(value, id, opt?.label ?? id, onFinishedEditing);
    },
    [onFinishedEditing, options, value]
  );

  return (
    <div className="w-[260px] max-w-[320px] box-border p-2">
      <div className="mb-1 text-[10px] font-medium text-muted-foreground">{title}</div>
      <Input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={placeholder ?? 'Search…'}
        className="h-8 px-2 py-1 text-xs focus:ring-0 focus:border-accent"
        onKeyDown={(e) => {
          if (flat.length === 0) {
            if (e.key === 'Escape') {
              e.preventDefault();
              onFinishedEditing(undefined);
            }
            return;
          }

          if (e.key === 'ArrowDown') {
            e.preventDefault();
            setHighlighted((h) => Math.min(flat.length - 1, h < 0 ? 0 : h + 1));
          } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            setHighlighted((h) => Math.max(0, h <= 0 ? 0 : h - 1));
          } else if (e.key === 'Enter') {
            e.preventDefault();
            const opt = flat[highlighted];
            if (opt) commit(opt.id);
          } else if (e.key === 'Escape') {
            e.preventDefault();
            onFinishedEditing(undefined);
          } else if (e.key === 'Home') {
            e.preventDefault();
            setHighlighted(0);
          } else if (e.key === 'End') {
            e.preventDefault();
            setHighlighted(flat.length - 1);
          } else if (e.key === 'PageDown') {
            e.preventDefault();
            setHighlighted((h) => Math.min(flat.length - 1, h + 10));
          } else if (e.key === 'PageUp') {
            e.preventDefault();
            setHighlighted((h) => Math.max(0, h - 10));
          }
        }}
      />

      <div className="mt-2 rounded-notion border border-border bg-background overflow-hidden">
        <div ref={listRef} className="max-h-[260px] overflow-auto p-1">
          {flat.length === 0 ? (
            <div className="px-2 py-2 text-xs text-muted-foreground">No matches</div>
          ) : (
            <>
              {grouped.sortedGroups.map((groupName) => (
                <div key={groupName}>
                  <div className="px-2 py-1.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                    {groupName}
                  </div>
                  {grouped.groups[groupName].map((opt) => {
                    const flatIndex = flat.indexOf(opt);
                    const isHighlighted = flatIndex === highlighted;
                    return (
                      <button
                        key={opt.id}
                        ref={isHighlighted ? highlightedItemRef : null}
                        type="button"
                        onClick={() => commit(opt.id)}
                        onMouseEnter={() => setHighlighted(flatIndex)}
                        className={[
                          'flex w-full items-center gap-2 px-2 py-1.5 text-left text-xs transition-colors rounded-notion',
                          'hover:bg-hover',
                          isHighlighted ? 'bg-hover' : '',
                        ].join(' ')}
                        style={opt.color ? { boxShadow: `inset 3px 0 0 ${opt.color}` } : undefined}
                      >
                        {opt.icon ? <span className="w-5 text-center">{opt.icon}</span> : null}
                        {!opt.icon && opt.color ? (
                          <span
                            className="inline-block h-2.5 w-2.5 rounded-full"
                            style={{ backgroundColor: opt.color }}
                          />
                        ) : null}
                        <div className="min-w-0 flex-1">
                          <div className="truncate">{opt.label}</div>
                          {opt.subLabel ? (
                            <div className="truncate text-[10px] text-muted-foreground">{opt.subLabel}</div>
                          ) : null}
                        </div>
                      </button>
                    );
                  })}
                </div>
              ))}
              {grouped.ungrouped.length > 0 ? (
                <div>
                  {grouped.sortedGroups.length > 0 ? (
                    <div className="px-2 py-1.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                      Other
                    </div>
                  ) : null}
                  {grouped.ungrouped.map((opt) => {
                    const flatIndex = flat.indexOf(opt);
                    const isHighlighted = flatIndex === highlighted;
                    return (
                      <button
                        key={opt.id}
                        ref={isHighlighted ? highlightedItemRef : null}
                        type="button"
                        onClick={() => commit(opt.id)}
                        onMouseEnter={() => setHighlighted(flatIndex)}
                        className={[
                          'flex w-full items-center gap-2 px-2 py-1.5 text-left text-xs transition-colors rounded-notion',
                          'hover:bg-hover',
                          isHighlighted ? 'bg-hover' : '',
                        ].join(' ')}
                        style={opt.color ? { boxShadow: `inset 3px 0 0 ${opt.color}` } : undefined}
                      >
                        {opt.icon ? <span className="w-5 text-center">{opt.icon}</span> : null}
                        {!opt.icon && opt.color ? (
                          <span
                            className="inline-block h-2.5 w-2.5 rounded-full"
                            style={{ backgroundColor: opt.color }}
                          />
                        ) : null}
                        <div className="min-w-0 flex-1">
                          <div className="truncate">{opt.label}</div>
                          {opt.subLabel ? (
                            <div className="truncate text-[10px] text-muted-foreground">{opt.subLabel}</div>
                          ) : null}
                        </div>
                      </button>
                    );
                  })}
                </div>
              ) : null}
            </>
          )}
        </div>
      </div>

      {footer ? <div className="mt-2">{footer}</div> : null}
    </div>
  );
}

export default function CSVImportDialog({
  isOpen,
  onClose,
  householdId,
  categories,
  onImportComplete,
}: CSVImportDialogProps) {
  const supabase = createClient();
  const { theme: appTheme, palette } = useAppTheme();
  const [fileName, setFileName] = useState('');
  const [headers, setHeaders] = useState<string[]>([]);
  const [rows, setRows] = useState<string[][]>([]);
  const [mapping, setMapping] = useState<ColumnMapping>({});
  const [transactions, setTransactions] = useState<EditableTransaction[]>([]);
  const [householdMembers, setHouseholdMembers] = useState<HouseholdMember[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [currentHouseholdName, setCurrentHouseholdName] = useState<string>('');
  const [selectedPayer, setSelectedPayer] = useState<string>('');
  const [currentUserId, setCurrentUserId] = useState<string>('');
  const [selectedHousehold, setSelectedHousehold] = useState<string>('');
  const [selectedAccount, setSelectedAccount] = useState<string>('');
  const [excludedRowIdxs, setExcludedRowIdxs] = useState<Set<number>>(() => new Set());
  const [showNewAccount, setShowNewAccount] = useState(false);
  const [newAccountName, setNewAccountName] = useState('');
  const [newAccountType, setNewAccountType] = useState<'chequing' | 'savings' | 'credit_card' | 'investment' | 'other'>('chequing');
  const [showNewHousehold, setShowNewHousehold] = useState(false);
  const [newHouseholdName, setNewHouseholdName] = useState('');
  const [presets, setPresets] = useState<CSVImportPreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState<CSVImportPreset | null>(null);
  const [autoMatchedPreset, setAutoMatchedPreset] = useState<CSVImportPreset | null>(null);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [vendorPrompt, setVendorPrompt] = useState<{
    transactionId: string;
    vendor: string;
    count: number;
    categoryId: string;
  } | null>(null);
  const [duplicates, setDuplicates] = useState<Map<string, Expense[]>>(new Map());
  const [showDuplicateDialog, setShowDuplicateDialog] = useState(false);
  const [excludedDuplicates, setExcludedDuplicates] = useState<Set<string>>(new Set());
  const [amountConflicts, setAmountConflicts] = useState<AmountConflict[]>([]);
  const [showAmountConflictDialog, setShowAmountConflictDialog] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [csvColumns, setCsvColumns] = useState<GridColumn[]>([]);
  const [scrollX, setScrollX] = useState(0);
  const dataGridContainerRef = useRef<HTMLDivElement>(null);
  const [dataGridSize, setDataGridSize] = useState<{ width: number; height: number }>({ width: 0, height: 0 });
  const reprocessTimerRef = useRef<number | null>(null);

  const unresolvedAmountConflicts = useMemo(() => {
    if (amountConflicts.length === 0) return [];
    return amountConflicts.filter((c) => !excludedRowIdxs.has(c.rowIdx));
  }, [amountConflicts, excludedRowIdxs]);

  useEffect(() => {
    if (isOpen && householdId) {
      fetchPresets();
      fetchCurrentUser();
      // Seed selection and load members for the current household
      setSelectedHousehold(householdId);
      fetchHouseholdMembers(householdId);
      fetchAccounts(householdId);
      fetchHouseholdName();
      if (householdId) {
        // handled above
      }
    }
  }, [isOpen, householdId]);

  // When the user changes the selected household (or creates a new one), refresh partner list.
  useEffect(() => {
    if (!isOpen) return;
    void fetchHouseholdMembers(selectedHousehold || householdId);
    void fetchAccounts(selectedHousehold || householdId);
  }, [isOpen, selectedHousehold, householdId]);

  // Validate that selectedPayer is a member of the selected household
  useEffect(() => {
    if (!isOpen || !selectedHousehold || !selectedPayer) return;
    if (householdMembers.length === 0) return; // Wait for members to load
    
    const isPayerValid = householdMembers.some(m => m.user_id === selectedPayer);
    if (!isPayerValid && householdMembers.length > 0) {
      // Reset to first member if current payer is not valid
      setSelectedPayer(householdMembers[0].user_id);
      // Update all transactions to use the new payer
      setTransactions((prev) => prev.map((t) => ({ ...t, payer_id: householdMembers[0].user_id })));
    }
  }, [isOpen, selectedHousehold, selectedPayer, householdMembers]);

  // Measure the available space for the Glide grid inside the modal so we can pass numeric width/height.
  useEffect(() => {
    if (!isOpen) return;
    const el = dataGridContainerRef.current;
    if (!el) return;

    const update = () => {
      const rect = el.getBoundingClientRect();
      const w = Math.floor(rect.width);
      const h = Math.floor(rect.height);
      if (w > 0 && h > 0) {
        setDataGridSize((prev) => (prev.width === w && prev.height === h ? prev : { width: w, height: h }));
      }
    };

    let raf = 0;
    const schedule = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(update);
    };

    // Run once now, and once next frame to catch flex/layout settling.
    schedule();
    requestAnimationFrame(update);

    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(() => schedule());
      ro.observe(el);
    }

    window.addEventListener('resize', schedule);
    return () => {
      window.removeEventListener('resize', schedule);
      if (ro) ro.disconnect();
      cancelAnimationFrame(raf);
    };
  }, [isOpen, headers.length]);

  const fetchHouseholdName = async () => {
    if (!householdId) return;
    try {
      const { data, error } = await supabase
        .from('households')
        .select('name')
        .eq('id', householdId)
        .single();
      if (error) throw error;
      setCurrentHouseholdName(data?.name || '');
    } catch (error) {
      console.error('Error fetching household name:', error);
    }
  };

  const fetchCurrentUser = async () => {
    const {
      data: { user },
    } = await supabase.auth.getUser();
    if (!user) return;
    setCurrentUserId(user.id);
    if (!selectedPayer) setSelectedPayer(user.id);
  };

  const fetchHouseholdMembers = async (targetHouseholdId: string | null | undefined) => {
    if (!targetHouseholdId || targetHouseholdId === 'new') {
      setHouseholdMembers([]);
      return;
    }
    try {
      // Prefer relationship join (works when the FK relationship is configured)
      const { data, error } = await supabase
        .from("household_members")
        .select(`
          user_id,
          profiles!inner (
            id,
            name
          )
        `)
        .eq("household_id", targetHouseholdId);

      if (error) {
        // Fallback: fetch member ids then profiles separately
        const { data: membersData, error: membersError } = await supabase
          .from("household_members")
          .select("user_id")
          .eq("household_id", targetHouseholdId);

        if (membersError) throw membersError;
        if (!membersData || membersData.length === 0) {
          setHouseholdMembers([]);
          return;
        }

        const userIds = membersData.map((m: any) => m.user_id);
        const { data: profilesData, error: profilesError } = await supabase
          .from("profiles")
          .select("id, name")
          .in("id", userIds);

        if (profilesError) throw profilesError;

        const profilesMap = new Map((profilesData || []).map((p: any) => [p.id, p]));
        const members = membersData.map((member: any) => {
          const profile = profilesMap.get(member.user_id);
          const fallbackName = `Member ${String(member.user_id).slice(0, 8)}`;
          return {
            user_id: member.user_id,
            profiles: {
              name: profile?.name ?? fallbackName,
              email: null,
            },
          };
        });

        setHouseholdMembers(members);
        return;
      }

      const members = (data || []).map((member: any) => {
        const profile = Array.isArray(member.profiles) ? member.profiles[0] : member.profiles;
        const fallbackName = `Member ${String(member.user_id).slice(0, 8)}`;
        return {
          user_id: member.user_id,
          profiles: {
            name: profile?.name ?? fallbackName,
            email: null,
          },
        };
      });

      setHouseholdMembers(members);
    } catch (error) {
      console.error('Error fetching household members:', error);
      setHouseholdMembers([]);
    }
  };

  const fetchAccounts = async (targetHouseholdId: string | null | undefined) => {
    if (!targetHouseholdId || targetHouseholdId === 'new') {
      setAccounts([]);
      return;
    }
    try {
      const { data, error } = await supabase
        .from('accounts')
        .select('id, name, type, user_id')
        .eq('household_id', targetHouseholdId)
        .order('name');
      if (error) throw error;
      setAccounts(data || []);
    } catch (error) {
      console.error('Error fetching accounts:', error);
      setAccounts([]);
    }
  };

  const fetchPresets = async () => {
    if (!householdId) return;
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;
      const { data, error } = await supabase
        .from('csv_import_presets')
        .select('*')
        .eq('user_id', user.id)
        .eq('household_id', householdId)
        .order('last_used_at', { ascending: false });
      if (error) throw error;
      setPresets(data || []);
    } catch (error) {
      console.error('Error fetching presets:', error);
    }
  };

  const handleFileUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setFileName(file.name);
    setLoading(true);

    try {
      const text = await file.text();
      if (!text || text.trim().length === 0) {
        alert('CSV file is empty');
        setLoading(false);
        return;
      }

      const parsed = parseCSV(text);
      if (!parsed || parsed.length === 0) {
        alert('CSV file could not be parsed or is empty');
        setLoading(false);
        return;
      }

      // Detect if first row looks like headers or data
      const firstRow = parsed[0];
      let fileHeaders: string[];
      let fileRows: string[][];
      
      // Check if first row contains actual data (dates, amounts) - if so, treat as data
      const datePattern = /^\d{1,2}[-\/]\d{1,2}[-\/]\d{2,4}$/;
      const amountPattern = /^-?\$?\d+\.?\d*$/;
      let firstRowHasData = false;
      if (firstRow) {
        for (const cell of firstRow) {
          const trimmed = cell.trim();
          const cleaned = trimmed.replace(/[,\s]/g, '');
          if (trimmed && (datePattern.test(trimmed) || amountPattern.test(cleaned))) {
            firstRowHasData = true;
            break;
          }
        }
      }
      
      // Also check if first row looks similar to other rows (if they exist)
      // If first row has same data patterns as other rows, it's data, not headers
      let firstRowMatchesOtherRows = false;
      if (parsed.length > 1 && firstRow) {
        const sampleRows = parsed.slice(1, Math.min(4, parsed.length)); // Check next 1-3 rows
        let firstRowDataCount = 0;
        let sampleRowsDataCount = 0;
        
        for (const cell of firstRow) {
          const trimmed = cell.trim();
          const cleaned = trimmed.replace(/[,\s]/g, '');
          if (trimmed && (datePattern.test(trimmed) || amountPattern.test(cleaned))) {
            firstRowDataCount++;
          }
        }
        
        for (const row of sampleRows) {
          for (const cell of row) {
            const trimmed = cell.trim();
            const cleaned = trimmed.replace(/[,\s]/g, '');
            if (trimmed && (datePattern.test(trimmed) || amountPattern.test(cleaned))) {
              sampleRowsDataCount++;
            }
          }
        }
        
        // If first row has similar data patterns to other rows, it's data
        if (firstRowDataCount > 0 && sampleRowsDataCount > 0) {
          firstRowMatchesOtherRows = true;
        }
      }
      
      // If first row has data-like values or matches other rows, it's definitely data (not headers)
      // Otherwise, check if it looks like headers
      if (!firstRowHasData && !firstRowMatchesOtherRows && firstRow && firstRow.length > 0 && looksLikeHeaders(firstRow)) {
        // First row is headers
        fileHeaders = firstRow;
        fileRows = parsed.slice(1);
      } else {
        // No headers - all rows are data, generate generic headers
        // Find the maximum number of columns across all rows
        const numColumns = parsed.length > 0 
          ? Math.max(...parsed.map(row => row.length))
          : 0;
        fileHeaders = Array.from({ length: numColumns }, (_, i) => `Column ${i + 1}`);
        fileRows = parsed; // Include all rows as data
      }

      if (!fileHeaders || fileHeaders.length === 0) {
        alert('CSV file has no columns');
        setLoading(false);
        return;
      }

      setHeaders(fileHeaders);
      setRows(fileRows);
      const initialExcluded = new Set<number>();
      setExcludedRowIdxs(initialExcluded);
      setCsvColumns([
        { id: '__include', title: 'Import', width: 76 },
        ...buildCsvColumns(fileHeaders, fileRows),
      ]);
      setScrollX(0);

      // Try to match preset
      const matchedPreset = presets.find((p) =>
        p.filename_pattern && matchFilenamePattern(file.name, p.filename_pattern)
      );

      let detectedMapping: ColumnMapping;
      if (matchedPreset) {
        detectedMapping = matchedPreset.column_mapping;
        setSelectedPreset(matchedPreset);
        setAutoMatchedPreset(matchedPreset);
      } else {
        detectedMapping = autoDetectColumnMapping(fileHeaders, fileRows);
        setAutoMatchedPreset(null);
      }

      setMapping(detectedMapping);
      await processTransactions(fileHeaders, fileRows, detectedMapping, {
        excludedRowIdxs: initialExcluded,
      });
    } catch (error: any) {
      console.error('Error parsing CSV:', error);
      alert(`Error parsing CSV file: ${error?.message || 'Unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  const processTransactions = async (
    fileHeaders: string[],
    fileRows: string[][],
    currentMapping: ColumnMapping,
    opts?: {
      excludedRowIdxs?: Set<number>;
    }
  ) => {
    const defaultPayerId = currentUserId || '';
    const excluded = opts?.excludedRowIdxs ?? excludedRowIdxs;
    const nextConflicts: AmountConflict[] = [];

    const processed: EditableTransaction[] = fileRows
      .map((row, index) => {
        if (excluded?.has(index)) return null;

        const getMappedHeaders = (field: string): string[] => {
          const raw = (currentMapping as any)[field] as string | string[] | undefined;
          return Array.isArray(raw) ? raw : raw ? [raw] : [];
        };

        const getMappedCandidates = (field: AmountConflictField): Array<{ header: string; raw: string; parsed: number | null }> => {
          const headersForField = getMappedHeaders(field);
          const out: Array<{ header: string; raw: string; parsed: number | null }> = [];
          for (const h of headersForField) {
            const idx = fileHeaders.indexOf(h);
            if (idx < 0) continue;
            const rawVal = String(row[idx] ?? '').trim();
            if (!rawVal) continue;
            const parsed = normalizeAmount(rawVal);
            // Treat 0 as "empty" for conflict detection/import.
            if (parsed === null || parsed === 0) continue;
            out.push({ header: h, raw: rawVal, parsed });
          }
          return out;
        };

        const getMappedValue = (field: string): string => {
          const raw = (currentMapping as any)[field] as string | string[] | undefined;
          const headersForField = Array.isArray(raw) ? raw : raw ? [raw] : [];
          for (const h of headersForField) {
            const idx = fileHeaders.indexOf(h);
            if (idx < 0) continue;
            const v = String(row[idx] ?? '');
            if (v.trim() !== '') return v;
          }
          return '';
        };

        const dateStr = getMappedValue('date');
        const amountStr = getMappedValue('amount');
        const expenseStr = getMappedValue('expense');
        const creditStr = getMappedValue('credit');
        const descriptionStr = getMappedValue('description');
        const vendorStr = currentMapping.vendor 
          ? String(row[fileHeaders.indexOf(currentMapping.vendor)] ?? '').trim()
          : '';
        const categoryStr = currentMapping.category ? row[fileHeaders.indexOf(currentMapping.category)] : '';
        const accountStr = getMappedValue('account');

        const date = normalizeDate(dateStr) || new Date().toISOString().split('T')[0];
        
        // Conflict detection:
        // If the user mapped multiple columns to the same field, ensure only one of those columns has a value per row.
        // Note: expense and credit are different fields, so having both mapped is fine.
        for (const field of ['amount', 'expense', 'credit'] as const) {
          const candidates = getMappedCandidates(field);
          // Only flag as conflict if multiple columns for the SAME field have values
          if (candidates.length > 1) {
            nextConflicts.push({
              rowIdx: index,
              field,
              columns: candidates.map((c) => c.header),
              values: candidates.map((c) => c.raw),
            });
          }
        }

        // If this row has any conflict, skip it (user must fix mapping or exclude the row).
        if (nextConflicts.some((c) => c.rowIdx === index)) {
          return null;
        }

        // Simple amount import: get amount from any mapped column and use absolute value
        // Category-based normalization will handle expense vs income after import
        let amount: number | null = null;
        
        // Try to get amount from expense column, credit column, or amount column
        if (expenseStr) {
          const expenseAmount = normalizeAmount(expenseStr);
          if (expenseAmount !== null && expenseAmount !== 0) {
            amount = expenseAmount; // Preserve negative values
          }
        }
        
        if (amount === null && creditStr) {
          const creditAmount = normalizeAmount(creditStr);
          if (creditAmount !== null && creditAmount !== 0) {
            amount = creditAmount; // Preserve negative values
          }
        }
        
        if (amount === null && amountStr) {
          const rawAmount = normalizeAmount(amountStr);
          if (rawAmount !== null && rawAmount !== 0) {
            amount = rawAmount; // Preserve negative values
          }
        }
        
        // If no amount found, skip this transaction
        if (amount === null || amount === 0) {
          return null;
        }
        
        // Extract vendor first (from vendor column or description column)
        const vendorRaw = vendorStr || descriptionStr.trim();
        const vendor = vendorRaw ? extractVendor(vendorRaw) : null;
        
        // Use description if available, otherwise use vendor as description
        // This ensures description is never empty when vendor exists
        const description = descriptionStr.trim() || vendor || '';

        // Find category by name if provided
        let category_id = '';
        if (categoryStr) {
          const foundCategory = categories.find(
            (c) => c.name.toLowerCase() === categoryStr.toLowerCase()
          );
          if (foundCategory) {
            category_id = foundCategory.id;
          }
        }

        // Use selectedPayer or defaultPayerId (partner is set via bottom bar, not from CSV)
        let payer_id = selectedPayer || defaultPayerId;

        // Find account by name if provided
        // Filter accounts by selected payer to ensure we only use accounts belonging to that payer
        let account_id: string | null = null;
        if (accountStr) {
          const payerAccounts = selectedPayer 
            ? accounts.filter(acc => !acc.user_id || acc.user_id === selectedPayer)
            : accounts;
          const foundAccount = payerAccounts.find(
            (acc) => acc.name.toLowerCase() === accountStr.toLowerCase()
          );
          if (foundAccount) {
            account_id = foundAccount.id;
          }
        }

        if (!date) {
          return null;
        }

        return {
          id: `row-${index}`,
          date,
          amount,
          description,
          category_id,
          payer_id,
          account_id,
          vendor,
        };
      })
      .filter((t): t is EditableTransaction => t !== null);

    setAmountConflicts(nextConflicts);

    // Prefill categories from vendor history
    if (selectedHousehold && processed.length > 0) {
      const vendors = processed.map((t) => t.vendor);
      const vendorCategoryMap = await getVendorCategories(supabase, selectedHousehold, vendors);
      
      // Apply category prefilling to transactions without categories
      const processedWithCategories = processed.map((t) => {
        if (t.category_id) {
          return t; // Already has a category
        }
        
        const normalizedVendor = normalizeVendor(t.vendor);
        if (normalizedVendor && vendorCategoryMap.has(normalizedVendor)) {
          return {
            ...t,
            category_id: vendorCategoryMap.get(normalizedVendor)!,
          };
        }
        
        return t;
      });
      
      setTransactions(processedWithCategories);
    } else {
      setTransactions(processed);
    }
    
    if (!selectedPayer && defaultPayerId) {
      setSelectedPayer(defaultPayerId);
    }
  };

  const handlePartnerChange = (payerId: string) => {
    setSelectedPayer(payerId);
    // Apply to all transactions
    setTransactions((prev) => prev.map((t) => ({ ...t, payer_id: payerId })));
    
    // Reset selected account if it doesn't belong to the new payer
    if (selectedAccount) {
      const account = accounts.find(acc => acc.id === selectedAccount);
      if (account && account.user_id && account.user_id !== payerId) {
        setSelectedAccount('');
      }
    }
  };

  const handleAutoDetect = async () => {
    const detected = autoDetectColumnMapping(headers, rows);
    setMapping(detected);
    await processTransactions(headers, rows, detected, {
      excludedRowIdxs,
    });
  };

  const applyCategoryToVendor = (vendor: string, categoryId: string) => {
    const normalizedVendor = normalizeVendor(vendor);
    setTransactions((prev) =>
      prev.map((t) =>
        normalizeVendor(t.vendor || '') === normalizedVendor && !t.category_id
          ? { ...t, category_id: categoryId }
          : t
      )
    );
    setVendorPrompt(null);
  };

  const createAccount = async () => {
    if (!newAccountName.trim()) return;
    if (!selectedHousehold || !selectedPayer) {
      alert('Please select a household and payer before creating an account');
      return;
    }
    try {
      // Validate that selectedPayer is a member of the household
      const isPayerValid = householdMembers.some(m => m.user_id === selectedPayer);
      if (!isPayerValid) {
        alert('The selected payer must be a member of the selected household');
        return;
      }
      
      const { data, error } = await supabase
        .from('accounts')
        .insert({
          user_id: selectedPayer, // Create account for the selected payer, not the current user
          household_id: selectedHousehold,
          name: newAccountName.trim(),
          type: newAccountType,
        })
        .select()
        .single();
      if (error) throw error;
      await fetchAccounts(selectedHousehold);
      if (data) {
        setSelectedAccount(data.id);
      }
      setShowNewAccount(false);
      setNewAccountName('');
    } catch (error) {
      console.error('Error creating account:', error);
      alert('Failed to create account');
    }
  };

  const createHousehold = async () => {
    if (!newHouseholdName.trim()) return;
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;
      const { data, error } = await supabase.rpc('create_household', {
        p_name: newHouseholdName.trim(),
      });
      if (error) throw error;
      if (data) {
        setSelectedHousehold(data.id);
        await fetchHouseholdMembers(data.id);
      }
      setShowNewHousehold(false);
      setNewHouseholdName('');
    } catch (error) {
      console.error('Error creating household:', error);
      alert('Failed to create household');
    }
  };

  const handleImport = async () => {
    if (!selectedHousehold || !selectedPayer) {
      alert('Please select a household and partner');
      return;
    }

    // Validate that the selected payer is a member of the selected household
    const isPayerValid = householdMembers.some(m => m.user_id === selectedPayer);
    if (!isPayerValid) {
      alert('The selected payer must be a member of the selected household. Please select a valid payer.');
      return;
    }

    if (!mapping.date) {
      alert('Please map a Date column before importing. Date is required for all transactions.');
      return;
    }

    if (unresolvedAmountConflicts.length > 0) {
      setShowAmountConflictDialog(true);
      return;
    }

    // Check for duplicates before importing
    const duplicateMap = await checkForDuplicates(supabase, selectedHousehold, transactions);
    
    if (duplicateMap.size > 0) {
      // Show duplicate dialog
      setDuplicates(duplicateMap);
      setExcludedDuplicates(new Set());
      setShowDuplicateDialog(true);
      return;
    }

    // No duplicates found, proceed with import
    await performImport();
  };

  const performImport = async () => {
    if (!selectedHousehold || !selectedPayer) {
      return;
    }

    // Validate that the selected payer is a member of the selected household
    const isPayerValid = householdMembers.some(m => m.user_id === selectedPayer);
    if (!isPayerValid) {
      alert('The selected payer must be a member of the selected household. Please select a valid payer.');
      setImporting(false);
      return;
    }

    setImporting(true);
    try {
      // Filter out excluded duplicates and ensure amounts are valid
      // Also ensure all transactions use a valid payer_id from the household
      const validPayerIds = new Set(householdMembers.map(m => m.user_id));
      
      // Get accounts that belong to the selected payer
      const payerAccounts = new Set(
        accounts
          .filter(acc => !acc.user_id || acc.user_id === selectedPayer)
          .map(acc => acc.id)
      );
      
      const transactionsToImport = transactions
        .filter((t) => !excludedDuplicates.has(t.id) && t.amount !== null && t.amount !== 0)
        .map((t, idx) => {
          // Ensure payer_id is valid - use selectedPayer if transaction payer is not valid
          const transactionPayerId = t.payer_id && validPayerIds.has(t.payer_id) 
            ? t.payer_id 
            : selectedPayer;
          
          // Validate account_id - ensure it belongs to the transaction payer
          let validAccountId: string | null = null;
          if (t.account_id) {
            // Check if the account belongs to the transaction payer
            const account = accounts.find(acc => acc.id === t.account_id);
            if (account && (!account.user_id || account.user_id === transactionPayerId)) {
              validAccountId = t.account_id;
            } else if (selectedAccount && payerAccounts.has(selectedAccount)) {
              // Fall back to selected account if it belongs to the payer
              validAccountId = selectedAccount;
            }
          } else if (selectedAccount && payerAccounts.has(selectedAccount)) {
            validAccountId = selectedAccount;
          }
          
          const result = {
            household_id: selectedHousehold,
            payer_id: transactionPayerId,
            account_id: validAccountId,
            // Preserve negative values from CSV for easier tracking
            amount: t.amount,
            date: t.date,
            description: t.description || undefined,
            vendor: t.vendor || undefined,
            category_id: t.category_id || undefined,
            status: 'approved' as const,
          };
          return result;
        });
      if (transactionsToImport.length === 0) {
        alert('No transactions to import');
        return;
      }

      // Save preset
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (user) {
      if (selectedPreset) {
        await supabase
          .from('csv_import_presets')
          .update({
            column_mapping: mapping,
              default_payer_id: selectedPayer,
            last_used_at: new Date().toISOString(),
          })
          .eq('id', selectedPreset.id);
      } else {
          await supabase.from('csv_import_presets').insert({
            user_id: user.id,
            household_id: selectedHousehold,
            preset_name: fileName,
            filename_pattern: fileName,
            column_mapping: mapping,
            default_payer_id: selectedPayer,
            last_used_at: new Date().toISOString(),
          });
        }
      }

      // Await so errors in the caller (DB insert) are caught and shown here.
      await Promise.resolve(onImportComplete(transactionsToImport));
      handleClose();
    } catch (error: any) {
      console.error('Error importing transactions:', error);
      alert('Error importing transactions');
    } finally {
      setImporting(false);
    }
  };

  const handleImportSelected = async () => {
    setShowDuplicateDialog(false);
    await performImport();
  };

  const handleImportAll = async () => {
    setExcludedDuplicates(new Set());
    setShowDuplicateDialog(false);
    await performImport();
  };

  const handleClose = () => {
    if (reprocessTimerRef.current) {
      window.clearTimeout(reprocessTimerRef.current);
      reprocessTimerRef.current = null;
    }
    setFileName('');
    setHeaders([]);
    setRows([]);
    setMapping({});
    setTransactions([]);
    setSelectedPreset(null);
    setAutoMatchedPreset(null);
    setSelectedPayer('');
    setCurrentUserId('');
    setSelectedAccount('');
    setShowNewAccount(false);
    setShowNewHousehold(false);
    setExcludedRowIdxs(new Set());
    setCsvColumns([]);
    setScrollX(0);
    setDuplicates(new Map());
    setShowDuplicateDialog(false);
    setExcludedDuplicates(new Set());
    onClose();
  };

  const hasFile = headers.length > 0;

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

    // Light mode needs more table contrast than the global Notion-like border.
    // Dark mode is already sufficiently contrasty.
    const gridBorder = appTheme === 'dark' ? border : withAlpha(foreground, 0.30);
    const gridBorderStrong = appTheme === 'dark' ? border : withAlpha(foreground, 0.40);

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

      borderColor: gridBorder,
      horizontalBorderColor: gridBorder,
      headerBottomBorderColor: gridBorderStrong,

      linkColor: accent,
    };
  }, [appTheme, palette]);

  // ---- M3-B (raw CSV columns + mapping row) ----

  const mappingOptions = useMemo<DropdownOption[]>(
    () => [
      { id: 'skip', label: 'Skip' },
      { id: 'date', label: 'Date' },
      { id: 'amount', label: 'Amount' },
      { id: 'expense', label: 'Expense' },
      { id: 'credit', label: 'Credit' },
      { id: 'description', label: 'Description' },
      { id: 'vendor', label: 'Vendor' },
      { id: 'category', label: 'Category' },
      { id: 'account', label: 'Account' },
    ],
    []
  );

  const headerToField = useMemo(() => {
    const m = new Map<string, keyof ColumnMapping>();
    for (const [k, v] of Object.entries(mapping)) {
      if (typeof v === 'string' && v.trim() !== '') {
        m.set(v, k as keyof ColumnMapping);
        continue;
      }
      if (Array.isArray(v)) {
        for (const header of v) {
          if (typeof header === 'string' && header.trim() !== '') {
            m.set(header, k as keyof ColumnMapping);
          }
        }
      }
    }
    return m;
  }, [mapping]);

  const mappedThemeOverride = useMemo<Partial<GlideTheme>>(() => {
    const defaults = getDefaultTheme();
    const accent = (glideTheme.accentColor ?? defaults.accentColor) as string;
    // Subtle tint so it's readable in both light/dark.
    const cellTint = withAlpha(accent, appTheme === 'dark' ? 0.14 : 0.09);
    const headerTint = withAlpha(accent, appTheme === 'dark' ? 0.22 : 0.14);
    const headerHoverTint = withAlpha(accent, appTheme === 'dark' ? 0.28 : 0.18);

    return {
      bgCell: cellTint,
      bgCellMedium: cellTint,
      bgHeader: headerTint,
      bgHeaderHovered: headerHoverTint,
      bgHeaderHasFocus: headerHoverTint,
    };
  }, [appTheme, glideTheme.accentColor]);

  const skippedThemeOverride = useMemo<Partial<GlideTheme>>(() => {
    const bg = appTheme === 'dark' 
      ? withAlpha(glideTheme.bgCell as string, 0.15) // Very dark in dark mode
      : withAlpha(glideTheme.bgCell as string, 0.5); // Lighter in light mode
    const textMuted = withAlpha(
      glideTheme.textMedium as string, 
      appTheme === 'dark' ? 0.3 : 0.4
    );
    return {
      bgCell: bg,
      bgCellMedium: bg,
      bgHeader: bg,
      bgHeaderHovered: bg,
      bgHeaderHasFocus: bg,
      textDark: textMuted,
      textHeader: textMuted,
    };
  }, [appTheme, glideTheme.bgCell, glideTheme.textMedium]);

  const prefixWidths = useMemo(() => {
    const out: number[] = [0];
    let sum = 0;
    for (const c of csvColumns) {
      sum += c.width ?? 0;
      out.push(sum);
    }
    return out;
  }, [csvColumns]);

  const handleVisibleRegionChanged = useCallback(
    (range: any, tx: number) => {
      const x = typeof range?.x === 'number' ? range.x : 0;
      const base = prefixWidths[x] ?? 0;
      const next = Math.max(0, Math.round(base + -tx));
      setScrollX((prev) => (Math.abs(prev - next) >= 2 ? next : prev));
    },
    [prefixWidths]
  );

  const handleColumnResize = useCallback((_: GridColumn, newSize: number, colIndex: number) => {
    setCsvColumns((prev) => prev.map((c, i) => (i === colIndex ? { ...c, width: newSize } : c)));
  }, []);

  const getMappingCellContent = useCallback(
    (cell: Item): CsvGridCell => {
      const [col, row] = cell;
      if (row !== 0) {
        return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
      }

      // Column 0 is a non-CSV "Import" checkbox column (row selection), not mappable.
      if (col === 0) {
        return {
          kind: GridCellKind.Text,
          data: '',
          displayData: '',
          allowOverlay: false,
          style: 'faded',
          themeOverride: skippedThemeOverride,
        } as CsvGridCell;
      }

      const header = headers[col - 1] ?? '';
      const field = header ? headerToField.get(header) ?? 'skip' : 'skip';
      const label = mappingOptions.find((o) => o.id === field)?.label ?? 'Skip';
      const isMapped = field !== 'skip';
      return {
        kind: GridCellKind.Text,
        data: field,
        displayData: label,
        allowOverlay: true,
        style: isMapped ? 'normal' : 'faded',
        themeOverride: isMapped ? mappedThemeOverride : skippedThemeOverride,
        csvEditor: 'mapping',
      };
    },
    [headers, headerToField, mappingOptions, mappedThemeOverride, skippedThemeOverride]
  );

  const drawSkipCell = useCallback<DrawCellCallback>((args, drawContent) => {
    drawContent();
    
    const cell = args.cell as CsvGridCell;
    const field = String(cell?.data ?? '');
    if (field !== 'skip') return;
    
    const { ctx, rect, theme } = args;
    const { x, y, width, height } = rect;
    
    ctx.save();
    ctx.strokeStyle = withAlpha(theme.textMedium as string, appTheme === 'dark' ? 0.3 : 0.4);
    ctx.lineWidth = 1;
    ctx.setLineDash([2, 2]);
    
    // Draw diagonal line from top-left to bottom-right
    ctx.beginPath();
    ctx.moveTo(x + 4, y + 4);
    ctx.lineTo(x + width - 4, y + height - 4);
    ctx.stroke();
    
    ctx.restore();
  }, [appTheme]);

  const drawMappingCell = useCallback<DrawCellCallback>((args, drawContent) => {
    drawContent();

    // Only decorate the mapping strip row (row 0).
    if (args.row !== 0) return;
    if (args.col === 0) return;

    const field = String((args.cell as any)?.data ?? '');
    if (!field || field === 'skip') {
      // Draw skip indicator for skip cells
      drawSkipCell(args, drawContent);
      return;
    }

    const { ctx, rect, theme } = args;

    const label = 'ASSIGNED AS';
    const fontSize = 8;
    const padX = 4;
    const padY = 2;
    const x = rect.x + 6;
    const y = rect.y + 5;

    ctx.save();
    ctx.textBaseline = 'middle';
    ctx.font = `600 ${fontSize}px ${theme.fontFamily}`;

    const textW = ctx.measureText(label).width;
    const badgeH = fontSize + padY * 2;
    const badgeW = Math.min(rect.width - 12, Math.ceil(textW + padX * 2));

    const r = 4;
    const bg = withAlpha(theme.accentColor, appTheme === 'dark' ? 0.22 : 0.16);
    const border = withAlpha(theme.accentColor, appTheme === 'dark' ? 0.45 : 0.60);

    ctx.fillStyle = bg;
    ctx.strokeStyle = border;
    ctx.lineWidth = 1;

    // Rounded rect path
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + badgeW - r, y);
    ctx.quadraticCurveTo(x + badgeW, y, x + badgeW, y + r);
    ctx.lineTo(x + badgeW, y + badgeH - r);
    ctx.quadraticCurveTo(x + badgeW, y + badgeH, x + badgeW - r, y + badgeH);
    ctx.lineTo(x + r, y + badgeH);
    ctx.quadraticCurveTo(x, y + badgeH, x, y + badgeH - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();

    ctx.fill();
    ctx.stroke();

    // In light mode, accentFg is often white which can be low-contrast on the accent-tinted badge.
    // Use the accent color itself for stronger readability.
    ctx.fillStyle = appTheme === 'dark' ? theme.accentFg : theme.accentColor;
    ctx.fillText(label, x + padX, y + badgeH / 2);
    ctx.restore();
  }, [appTheme]);

  const provideMappingEditor = useCallback(
    (cell: GridCell) => {
      const editor = (cell as CsvGridCell).csvEditor;
      if (editor !== 'mapping') return undefined;
      return (p: any) => {
        return (
          <GridDropdownEditor
            {...p}
            title="Map column to"
            options={mappingOptions}
            placeholder="Select field…"
          />
        );
      };
    },
    [mappingOptions]
  );

  const handleMappingCellsEdited = useCallback(
    (items: readonly EditListItem[]) => {
      if (items.length === 0) return true;

      const nextMapping: ColumnMapping = { ...mapping };

      for (const it of items) {
        const [col, row] = it.location;
        if (row !== 0) continue;
        if (col === 0) continue;
        const header = headers[col - 1];
        if (!header) continue;

        const selected = String((it.value as any)?.data ?? 'skip').trim() || 'skip';

        // Remove this header from any existing mappings (supports string or string[])
        for (const key of Object.keys(nextMapping)) {
          const val = (nextMapping as any)[key] as string | string[] | undefined;
          if (val === header) {
            delete (nextMapping as any)[key];
            continue;
          }
          if (Array.isArray(val)) {
            const filtered = val.filter((h) => h !== header);
            if (filtered.length === 0) delete (nextMapping as any)[key];
            else if (filtered.length === 1) (nextMapping as any)[key] = filtered[0];
            else (nextMapping as any)[key] = filtered;
          }
        }

        if (selected !== 'skip') {
          // Allow multiple headers for the same field (string[]), e.g. multiple debit/credit columns.
          const existing = (nextMapping as any)[selected] as string | string[] | undefined;
          if (!existing) {
            (nextMapping as any)[selected] = header;
          } else if (Array.isArray(existing)) {
            if (!existing.includes(header)) {
              (nextMapping as any)[selected] = [...existing, header];
            }
          } else if (existing !== header) {
            (nextMapping as any)[selected] = [existing, header];
          }
        }
      }

      scheduleMicrotask(() => {
        setMapping(nextMapping);
        void processTransactions(headers, rows, nextMapping, {
          excludedRowIdxs,
        });
      });

      return true;
    },
    [headers, mapping, rows, excludedRowIdxs]
  );

  const getRawCellContent = useCallback(
    (cell: Item): GridCell => {
      const [col, rowIdx] = cell;
      if (col === 0) {
        return {
          kind: GridCellKind.Boolean,
          data: !excludedRowIdxs.has(rowIdx),
          allowOverlay: false,
          style: 'normal',
          themeOverride: skippedThemeOverride,
        } as any;
      }

      const v = String(rows[rowIdx]?.[col - 1] ?? '');
      const header = headers[col - 1];
      const isMapped = Boolean(header && headerToField.has(header));
      const isExcluded = excludedRowIdxs.has(rowIdx);
      return {
        kind: GridCellKind.Text,
        data: v,
        displayData: v,
        allowOverlay: true,
        style: isExcluded ? 'faded' : isMapped ? 'normal' : 'faded',
        themeOverride: isExcluded ? skippedThemeOverride : isMapped ? mappedThemeOverride : skippedThemeOverride,
      };
    },
    [rows, headers, headerToField, mappedThemeOverride, skippedThemeOverride, excludedRowIdxs]
  );

  const handleRawCellsEdited = useCallback(
    (items: readonly EditListItem[]) => {
      if (items.length === 0) return true;

      // Apply edits to raw rows (batch), then reprocess standardized transactions.
      const nextRows = [...rows];
      let nextExcluded = excludedRowIdxs;
      let excludedChanged = false;

      for (const it of items) {
        const [col, rowIdx] = it.location;
        if (rowIdx < 0 || rowIdx >= nextRows.length) continue;

        if (col === 0) {
          if (it.value.kind !== GridCellKind.Boolean) continue;
          const include = Boolean((it.value as any).data);
          if (!excludedChanged) {
            nextExcluded = new Set(excludedRowIdxs);
            excludedChanged = true;
          }
          if (include) nextExcluded.delete(rowIdx);
          else nextExcluded.add(rowIdx);
          continue;
        }

        const row = [...(nextRows[rowIdx] ?? [])];
        row[col - 1] = String((it.value as any)?.data ?? '');
        nextRows[rowIdx] = row;
      }

      scheduleMicrotask(() => {
        setRows(nextRows);
        if (excludedChanged) setExcludedRowIdxs(nextExcluded);

        if (reprocessTimerRef.current) {
          window.clearTimeout(reprocessTimerRef.current);
        }
        reprocessTimerRef.current = window.setTimeout(() => {
          void processTransactions(headers, nextRows, mapping, {
            excludedRowIdxs: nextExcluded,
          });
        }, 200);
      });

      return true;
    },
    [headers, mapping, rows, excludedRowIdxs]
  );

  if (!isOpen) return null;

  const portalEl = typeof document === 'undefined' ? null : document.getElementById('portal');

  const modal = (
    <>
      <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 p-4">
        <Card className="w-full max-w-5xl max-h-[98vh] overflow-hidden flex flex-col relative">
          <CardHeader className="flex-shrink-0">
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>Import CSV</CardTitle>
                <CardDescription>Upload, map columns, and edit transactions in one view</CardDescription>
              </div>
              <button onClick={handleClose} className="rounded p-1 hover:bg-hover" aria-label="Close">
                <X className="h-5 w-5" />
              </button>
            </div>
          </CardHeader>

          <CardContent className="flex-1 overflow-hidden flex flex-col">
            {!hasFile ? (
              // Upload Section
              <div className="space-y-4 flex-1 flex items-center justify-center">
                <div className="border-2 border-dashed border-border rounded-md p-8 text-center w-full max-w-md">
                  <Upload className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
                  <input
                    ref={fileInputRef}
                    type="file"
                    accept=".csv"
                    onChange={handleFileUpload}
                    className="hidden"
                    id="csv-file-input"
                  />
                  <Button type="button" onClick={() => fileInputRef.current?.click()} disabled={loading} size="sm" className="h-8 text-xs">
                    <FileText className="mr-2 h-3.5 w-3.5" />
                    {loading ? 'Processing...' : 'Choose CSV File'}
                  </Button>
                  <p className="mt-2 text-xs text-muted-foreground">Select a CSV file to import expenses</p>
                </div>
              </div>
            ) : (
              <>
                {/* Requirement Indicators */}
                <div className="flex-shrink-0 border-b border-border bg-muted/30 px-2 pt-0 pb-0">
                  <div className="flex items-center justify-center gap-3 flex-wrap">
                    {/* Partner Indicator */}
                    <div className="flex flex-col items-center gap-1">
                      <div className={`w-[22px] h-[22px] rounded-full flex items-center justify-center ${
                        selectedPayer 
                          ? 'bg-green-500 text-white' 
                          : 'bg-muted border-2 border-border'
                      }`}>
                        {selectedPayer ? <Check className="h-4 w-4" /> : null}
                      </div>
                      <span className="text-[10px] text-muted-foreground">Partner</span>
                    </div>
                    
                    {/* Household Indicator */}
                    <div className="flex flex-col items-center gap-1">
                      <div className={`w-[22px] h-[22px] rounded-full flex items-center justify-center ${
                        selectedHousehold 
                          ? 'bg-green-500 text-white' 
                          : 'bg-muted border-2 border-border'
                      }`}>
                        {selectedHousehold ? <Check className="h-4 w-4" /> : null}
                      </div>
                      <span className="text-[10px] text-muted-foreground">Household</span>
                    </div>
                    
                    {/* Account Indicator */}
                    <div className="flex flex-col items-center gap-1">
                      <div className={`w-[22px] h-[22px] rounded-full flex items-center justify-center ${
                        selectedAccount 
                          ? 'bg-green-500 text-white' 
                          : 'bg-muted border-2 border-border'
                      }`}>
                        {selectedAccount ? <Check className="h-4 w-4" /> : null}
                      </div>
                      <span className="text-[10px] text-muted-foreground">Account</span>
                    </div>
                    
                    {/* Date Indicator */}
                    <div className="flex flex-col items-center gap-1">
                      <div className={`w-[22px] h-[22px] rounded-full flex items-center justify-center ${
                        mapping.date 
                          ? 'bg-green-500 text-white' 
                          : 'bg-muted border-2 border-border'
                      }`}>
                        {mapping.date ? <Check className="h-4 w-4" /> : null}
                      </div>
                      <span className="text-[10px] text-muted-foreground">Date</span>
                    </div>
                    
                    {/* Amount Indicator */}
                    <div className="flex flex-col items-center gap-1">
                      <div className={`w-[22px] h-[22px] rounded-full flex items-center justify-center ${
                        mapping.amount || mapping.expense || mapping.credit
                          ? 'bg-green-500 text-white' 
                          : 'bg-muted border-2 border-border'
                      }`}>
                        {(mapping.amount || mapping.expense || mapping.credit) ? <Check className="h-4 w-4" /> : null}
                      </div>
                      <span className="text-[10px] text-muted-foreground">Amount</span>
                    </div>
                    
                    {/* Vendor Indicator */}
                    <div className="flex flex-col items-center gap-1">
                      <div className={`w-[22px] h-[22px] rounded-full flex items-center justify-center ${
                        mapping.vendor 
                          ? 'bg-green-500 text-white' 
                          : 'bg-muted border-2 border-border'
                      }`}>
                        {mapping.vendor ? <Check className="h-4 w-4" /> : null}
                      </div>
                      <span className="text-[10px] text-muted-foreground">Vendor</span>
                    </div>
                    
                    {/* Description Indicator (Optional) */}
                    <div className="flex flex-col items-center gap-1">
                      <div className={`w-[22px] h-[22px] rounded-full flex items-center justify-center ${
                        mapping.description 
                          ? 'bg-green-500 text-white' 
                          : 'bg-muted border-2 border-border'
                      }`}>
                        {mapping.description ? <Check className="h-4 w-4" /> : null}
                      </div>
                      <span className="text-[10px] text-muted-foreground">Description</span>
                    </div>
                  </div>
                </div>

                {/* S1 + M3-B: mapping grid (1 row) + raw data grid (scrolling) */}
                <div className="flex-shrink-0 border-b border-border bg-muted/30 p-2">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <Button variant="ghost" size="sm" onClick={handleAutoDetect} className="h-6 text-[10px]">
                      <Wand2 className="mr-1 h-2.5 w-2.5" />
                      Auto-detect
                    </Button>
                    <div className="text-[10px] text-muted-foreground">
                      {rows.length} row{rows.length !== 1 ? 's' : ''} • {headers.length} column{headers.length !== 1 ? 's' : ''} • {excludedRowIdxs.size} excluded • {transactions.length} importable
                    </div>
                  </div>
                </div>

                {/* Mapping strip (one row) */}
                <div className="flex-shrink-0 overflow-hidden rounded-md border border-border">
                  <DataEditor
                    columns={csvColumns}
                    rows={1}
                    getCellContent={getMappingCellContent}
                    onCellsEdited={handleMappingCellsEdited}
                    provideEditor={provideMappingEditor}
                    theme={glideTheme}
                    getCellsForSelection={true}
                    drawCell={drawMappingCell}
                    width={dataGridSize.width > 0 ? dataGridSize.width : '100%'}
                    height={84}
                    rowHeight={44}
                    headerHeight={28}
                    rowMarkers="none"
                    freezeColumns={0}
                    smoothScrollX={true}
                    scrollOffsetX={scrollX}
                    onVisibleRegionChanged={handleVisibleRegionChanged as any}
                    cellActivationBehavior="single-click"
                    editOnType={true}
                    onColumnResize={handleColumnResize}
                  />
                </div>

                {/* Raw CSV data grid (scrolling) */}
                <div
                  ref={dataGridContainerRef}
                  className="flex-1 min-h-0 overflow-hidden rounded-md border border-border"
                >
                  <DataEditor
                    columns={csvColumns}
                    rows={rows.length}
                    getCellContent={getRawCellContent}
                    onCellsEdited={handleRawCellsEdited}
                    theme={glideTheme}
                    getCellsForSelection={true}
                    drawCell={drawSkipCell}
                    // Fixed height so the grid is the scroll container (S1)
                    width={dataGridSize.width > 0 ? dataGridSize.width : '100%'}
                    height={dataGridSize.height > 0 ? dataGridSize.height : 600}
                    rowHeight={36}
                    headerHeight={0}
                    rowMarkers="none"
                    freezeColumns={0}
                    smoothScrollX={true}
                    scrollOffsetX={scrollX}
                    onVisibleRegionChanged={handleVisibleRegionChanged as any}
                    fillHandle={true}
                    allowedFillDirections="orthogonal"
                    cellActivationBehavior="single-click"
                    editOnType={true}
                    onColumnResize={handleColumnResize}
                  />
                </div>
              </>
            )}
          </CardContent>

          {/* Footer actions (inside the modal so it's never covered by the app footer) */}
          {hasFile && (
            <div className="flex-shrink-0 border-t border-border bg-background/80 backdrop-blur-md">
              <div className="px-4 py-3">
                <div className="grid gap-3 sm:grid-cols-4 sm:items-end">
                  <div className="min-w-0">
                    <label className="mb-1 block text-[10px] font-medium text-muted-foreground">Partner</label>
                    <Select
                      value={selectedPayer}
                      onChange={(e) => handlePartnerChange(e.target.value)}
                      className="w-full h-8 text-[10px] leading-tight"
                    >
                      <option value="">Select partner...</option>
                      {householdMembers.map((member) => (
                        <option key={member.user_id} value={member.user_id}>
                          {member.profiles?.name || member.profiles?.email || 'Unknown'}
                        </option>
                      ))}
                    </Select>
                  </div>

                  <div className="min-w-0">
                    <label className="mb-1 block text-[10px] font-medium text-muted-foreground">Household</label>
                    <Select
                      value={selectedHousehold}
                      onChange={(e) => {
                        setSelectedHousehold(e.target.value);
                        if (e.target.value === 'new') {
                          setShowNewHousehold(true);
                          setShowNewAccount(false);
                        } else {
                          setShowNewHousehold(false);
                        }
                      }}
                      className="w-full h-8 text-[10px] leading-tight"
                    >
                      <option value="">Select household...</option>
                      {householdId && currentHouseholdName && (
                        <option value={householdId}>{currentHouseholdName} (current)</option>
                      )}
                      <option value="new">+ Create New Household</option>
                    </Select>
                  </div>

                  <div className="min-w-0">
                    <label className="mb-1 block text-[10px] font-medium text-muted-foreground">Account</label>
                    <Select
                      value={selectedAccount}
                      onChange={(e) => {
                        setSelectedAccount(e.target.value);
                        if (e.target.value === 'new') {
                          setShowNewAccount(true);
                          setShowNewHousehold(false);
                        } else {
                          setShowNewAccount(false);
                        }
                      }}
                      className="w-full h-8 text-[10px] leading-tight"
                    >
                      <option value="">Select account...</option>
                      {accounts
                        .filter(acc => !selectedPayer || !acc.user_id || acc.user_id === selectedPayer)
                        .map((account) => (
                          <option key={account.id} value={account.id}>
                            {account.name} ({account.type})
                          </option>
                        ))}
                      <option value="new">+ Create New Account</option>
                    </Select>
                  </div>


                  <div className="flex gap-2 sm:justify-end sm:self-end">
                    <Button variant="ghost" onClick={handleClose} size="sm" className="h-8 text-xs">
                      Cancel
                    </Button>
                    {unresolvedAmountConflicts.length > 0 && (
                      <Button
                        variant="secondary"
                        onClick={() => setShowAmountConflictDialog(true)}
                        size="sm"
                        className="h-8 text-xs"
                        title="Resolve amount conflicts before importing"
                      >
                        Resolve Conflicts ({unresolvedAmountConflicts.length})
                      </Button>
                    )}
                    <Button
                      onClick={handleImport}
                      disabled={
                        importing ||
                        !selectedHousehold ||
                        !selectedPayer ||
                        !mapping.date ||
                        transactions.length === 0 ||
                        unresolvedAmountConflicts.length > 0
                      }
                      size="sm"
                      className="h-8 text-xs"
                    >
                      {importing ? 'Importing...' : `Import ${transactions.length}`}
                    </Button>
                  </div>

                  {showNewHousehold && (
                    <div className="sm:col-span-4 rounded-md border border-border bg-card p-3">
                      <div className="grid gap-2 sm:grid-cols-4 sm:items-end">
                        <div className="sm:col-span-3">
                          <label className="mb-1 block text-[10px] font-medium text-muted-foreground">Household name</label>
                          <Input
                            value={newHouseholdName}
                            onChange={(e) => setNewHouseholdName(e.target.value)}
                            placeholder="Household name"
                            className="w-full h-8 text-xs"
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') {
                                createHousehold();
                              }
                              if (e.key === 'Escape') {
                                setShowNewHousehold(false);
                                setNewHouseholdName('');
                              }
                            }}
                            autoFocus
                          />
                        </div>
                        <div className="flex gap-2">
                          <Button size="sm" onClick={createHousehold} className="h-8 flex-1 text-xs">
                            <Check className="mr-1 h-2.5 w-2.5" />
                            Create
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => {
                              setShowNewHousehold(false);
                              setNewHouseholdName('');
                            }}
                            className="h-8 text-xs"
                            title="Cancel"
                          >
                            <X className="h-2.5 w-2.5" />
                          </Button>
                        </div>
                      </div>
                    </div>
                  )}

                  {showNewAccount && (
                    <div className="sm:col-span-4 rounded-md border border-border bg-card p-3">
                      <div className="grid gap-2 sm:grid-cols-5 sm:items-end">
                        <div className="sm:col-span-2">
                          <label className="mb-1 block text-[10px] font-medium text-muted-foreground">Account name</label>
                          <Input
                            value={newAccountName}
                            onChange={(e) => setNewAccountName(e.target.value)}
                            placeholder="Account name"
                            className="w-full h-8 text-xs"
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') {
                                createAccount();
                              }
                              if (e.key === 'Escape') {
                                setShowNewAccount(false);
                                setNewAccountName('');
                              }
                            }}
                            autoFocus
                          />
                        </div>
                        <div className="sm:col-span-2">
                          <label className="mb-1 block text-[10px] font-medium text-muted-foreground">Type</label>
                          <Select
                            value={newAccountType}
                            onChange={(e) =>
                              setNewAccountType(
                                e.target.value as 'chequing' | 'savings' | 'credit_card' | 'investment' | 'other'
                              )
                            }
                            className="w-full h-8 text-[10px] leading-tight"
                          >
                            <option value="chequing">Chequing</option>
                            <option value="savings">Savings</option>
                            <option value="credit_card">Credit Card</option>
                            <option value="investment">Investment</option>
                            <option value="other">Other</option>
                          </Select>
                        </div>
                        <div className="flex gap-2">
                          <Button size="sm" onClick={createAccount} className="h-8 flex-1 text-xs">
                            <Check className="mr-1 h-2.5 w-2.5" />
                            Create
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => {
                              setShowNewAccount(false);
                              setNewAccountName('');
                            }}
                            className="h-8 text-xs"
                            title="Cancel"
                          >
                            <X className="h-2.5 w-2.5" />
                          </Button>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* Amount Conflicts Dialog */}
          {showAmountConflictDialog && (
            <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/50 p-4">
              <Card className="w-full max-w-3xl max-h-[90vh] overflow-hidden flex flex-col">
                <CardHeader className="flex-shrink-0">
                  <div className="flex items-center justify-between">
                    <div>
                      <CardTitle>Conflicting Amount Columns</CardTitle>
                      <CardDescription>
                        Some rows have more than one value across the mapped {`amount/expense/credit`} columns. Fix your column mapping or exclude those rows.
                      </CardDescription>
                    </div>
                    <button
                      onClick={() => setShowAmountConflictDialog(false)}
                      className="rounded p-1 hover:bg-hover"
                      aria-label="Close"
                    >
                      <X className="h-5 w-5" />
                    </button>
                  </div>
                </CardHeader>
                <CardContent className="flex-1 overflow-y-auto space-y-3">
                  {unresolvedAmountConflicts.length === 0 ? (
                    <div className="text-sm text-muted-foreground">No unresolved conflicts.</div>
                  ) : (
                    unresolvedAmountConflicts.slice(0, 200).map((c, idx) => (
                      <div key={`${c.rowIdx}-${c.field}-${idx}`} className="rounded-md border border-border bg-card p-3">
                        <div className="text-sm">
                          <span className="font-medium">Row:</span> {c.rowIdx + 1}{' '}
                          <span className="font-medium ml-3">Field:</span> {c.field}
                        </div>
                        <div className="mt-1 text-xs text-muted-foreground">
                          {c.columns.map((col, i) => (
                            <div key={`${col}-${i}`}>
                              <span className="font-medium">{col}:</span> {c.values[i]}
                            </div>
                          ))}
                        </div>
                      </div>
                    ))
                  )}
                  {unresolvedAmountConflicts.length > 200 && (
                    <div className="text-xs text-muted-foreground">
                      Showing first 200 conflicts. Exclude them in bulk or refine mapping to reduce conflicts.
                    </div>
                  )}
                </CardContent>
                <div className="flex-shrink-0 border-t border-border bg-background/80 backdrop-blur-md px-6 py-4">
                  <div className="flex items-center justify-between gap-3">
                    <div className="text-sm text-muted-foreground">
                      {unresolvedAmountConflicts.length} conflict{unresolvedAmountConflicts.length !== 1 ? 's' : ''} must be resolved to import.
                    </div>
                    <div className="flex gap-2">
                      <Button variant="ghost" onClick={() => setShowAmountConflictDialog(false)}>
                        Close
                      </Button>
                      <Button
                        variant="secondary"
                        onClick={() => {
                          const next = new Set(excludedRowIdxs);
                          for (const c of unresolvedAmountConflicts) next.add(c.rowIdx);
                          setExcludedRowIdxs(next);
                          setShowAmountConflictDialog(false);
                        }}
                      >
                        Exclude Conflicting Rows
                      </Button>
                    </div>
                  </div>
                </div>
              </Card>
            </div>
          )}
        </Card>
      </div>

      {/* Vendor Prompt Modal */}
      {vendorPrompt && (
        <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/50 p-4">
          <Card className="w-full max-w-md">
            <CardHeader>
              <CardTitle>Apply Category to Similar Transactions?</CardTitle>
              <CardDescription>
                Found {vendorPrompt.count} other transaction{vendorPrompt.count !== 1 ? 's' : ''} with vendor "
                {vendorPrompt.vendor}"
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="text-sm text-muted-foreground">
                Would you like to apply this category to all {vendorPrompt.count} transaction{vendorPrompt.count !== 1 ? 's' : ''} from this vendor?
              </div>
              <div className="flex gap-2 justify-end">
                <Button variant="ghost" onClick={() => setVendorPrompt(null)}>
                  No, just this one
                </Button>
                <Button onClick={() => applyCategoryToVendor(vendorPrompt.vendor, vendorPrompt.categoryId)}>
                  Yes, apply to all
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Duplicate Detection Dialog */}
      {showDuplicateDialog && (
        <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/50 p-4">
          <Card className="w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
            <CardHeader className="flex-shrink-0">
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle>Potential Duplicate Transactions</CardTitle>
                  <CardDescription>
                    Found {duplicates.size} transaction{duplicates.size !== 1 ? 's' : ''} that may already exist in your database
                  </CardDescription>
                </div>
                <button
                  onClick={() => setShowDuplicateDialog(false)}
                  className="rounded p-1 hover:bg-hover"
                  aria-label="Close"
                >
                  <X className="h-5 w-5" />
                </button>
              </div>
            </CardHeader>
            <CardContent className="flex-1 overflow-y-auto space-y-4">
              {Array.from(duplicates.entries()).map(([transactionId, existingExpenses]) => {
                const transaction = transactions.find((t) => t.id === transactionId);
                if (!transaction) return null;

                const isExcluded = excludedDuplicates.has(transactionId);

                return (
                  <div
                    key={transactionId}
                    className={`rounded-md border p-4 ${
                      isExcluded ? 'bg-muted/30 border-muted' : 'bg-card border-border'
                    }`}
                  >
                    <div className="flex items-start gap-4">
                      <input
                        type="checkbox"
                        checked={!isExcluded}
                        onChange={(e) => {
                          const newExcluded = new Set(excludedDuplicates);
                          if (e.target.checked) {
                            newExcluded.delete(transactionId);
                          } else {
                            newExcluded.add(transactionId);
                          }
                          setExcludedDuplicates(newExcluded);
                        }}
                        className="mt-1"
                      />
                      <div className="flex-1 grid grid-cols-2 gap-4">
                        {/* Imported Transaction */}
                        <div>
                          <div className="text-xs font-medium text-muted-foreground mb-2">Imported Transaction</div>
                          <div className="space-y-1 text-sm">
                            <div>
                              <span className="font-medium">Date:</span> {formatDate(transaction.date, 'MMM dd, yyyy')}
                            </div>
                            <div>
                              <span className="font-medium">Amount:</span> {formatCurrency(transaction.amount)}
                            </div>
                            {transaction.vendor && (
                              <div>
                                <span className="font-medium">Vendor:</span> {transaction.vendor}
                              </div>
                            )}
                            {transaction.description && (
                              <div>
                                <span className="font-medium">Description:</span> {transaction.description}
                              </div>
                            )}
                          </div>
                        </div>

                        {/* Existing Transaction(s) */}
                        <div>
                          <div className="text-xs font-medium text-muted-foreground mb-2">
                            Existing Transaction{existingExpenses.length > 1 ? 's' : ''}
                          </div>
                          <div className="space-y-3">
                            {existingExpenses.map((expense) => (
                              <div key={expense.id} className="space-y-1 text-sm border-l-2 border-accent pl-2">
                                <div>
                                  <span className="font-medium">Date:</span> {formatDate(expense.date, 'MMM dd, yyyy')}
                                </div>
                                <div>
                                  <span className="font-medium">Amount:</span> {formatCurrency(Number(expense.amount))}
                                </div>
                                {expense.vendor && (
                                  <div>
                                    <span className="font-medium">Vendor:</span> {expense.vendor}
                                  </div>
                                )}
                                {expense.description && (
                                  <div>
                                    <span className="font-medium">Description:</span> {expense.description}
                                  </div>
                                )}
                              </div>
                            ))}
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </CardContent>
            <div className="flex-shrink-0 border-t border-border bg-background/80 backdrop-blur-md px-6 py-4">
              <div className="flex items-center justify-between">
                <div className="text-sm text-muted-foreground">
                  {transactions.length - excludedDuplicates.size} of {transactions.length} transaction
                  {transactions.length !== 1 ? 's' : ''} will be imported
                </div>
                <div className="flex gap-2">
                  <Button variant="ghost" onClick={() => setShowDuplicateDialog(false)}>
                    Cancel
                  </Button>
                  <Button variant="secondary" onClick={handleImportSelected}>
                    Import Selected ({transactions.length - excludedDuplicates.size})
                  </Button>
                  <Button onClick={handleImportAll}>Import All</Button>
                </div>
              </div>
            </div>
          </Card>
        </div>
      )}
    </>
  );

  return portalEl ? createPortal(modal, portalEl) : modal;
}
