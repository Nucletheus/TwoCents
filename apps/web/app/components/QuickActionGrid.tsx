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
import type { Category } from '@twocents/shared';
import { formatCurrency, formatDate, generateColorVariations, getCategoryColor } from '@twocents/shared';
import { useTheme as useAppTheme } from './ThemeProvider';
import GridDateEditor from './GridDateEditor';
import GridDropdownEditor, { type DropdownOption } from './GridDropdownEditor';

type TwocentsEditorKind = 'date' | 'category' | 'payer' | 'account';

type TwocentsCellMeta = {
  twocentsEditor?: TwocentsEditorKind;
};

type TwocentsGridCell = GridCell & TwocentsCellMeta;

type QuickActionRow = {
  id: string;
  date: string;
  amount: number | null;
  category_id: string;
  description: string;
  payer_id: string;
  account_id: string | null;
};

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

interface QuickActionGridProps {
  categories: Category[];
  householdMembers: HouseholdMember[];
  accounts: Account[];
  selectedHouseholdId: string | null;
  defaultPayerId: string | null;
  onSave: (rows: Array<{
    date: string;
    amount: number;
    category_id: string;
    description?: string;
    payer_id: string;
    account_id?: string | null;
  }>) => Promise<void>;
}

function scheduleMicrotask(cb: () => void) {
  if (typeof queueMicrotask === 'function') queueMicrotask(cb);
  else Promise.resolve().then(cb);
}

function generateRowId() {
  return `temp-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

export default function QuickActionGrid({
  categories,
  householdMembers,
  accounts,
  selectedHouseholdId,
  defaultPayerId,
  onSave,
}: QuickActionGridProps) {
  const { theme: appTheme } = useAppTheme();
  const [rows, setRows] = useState<QuickActionRow[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [gridSelection, setGridSelection] = useState<GridSelection>(() => ({
    columns: CompactSelection.empty(),
    rows: CompactSelection.empty(),
  }));
  const gridRef = useRef<HTMLDivElement>(null);

  // Initialize rows when component mounts or when defaultPayerId becomes available
  useEffect(() => {
    if (rows.length === 0) {
      const today = new Date().toISOString().split('T')[0];
      setRows(Array.from({ length: 1 }, () => ({
        id: generateRowId(),
        date: today,
        amount: null,
        category_id: '',
        description: '',
        payer_id: defaultPayerId || '',
        account_id: null,
      })));
    }
  }, [defaultPayerId, rows.length]);

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
  }, [appTheme]);

  const rowHeight = 36;
  const headerHeight = 36;
  const gridHeight = headerHeight + rows.length * rowHeight + 20;

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
    return accounts.map((a) => ({
      id: a.id,
      label: a.name,
      subLabel: a.type,
    }));
  }, [accounts]);

  const glideColumns = useMemo<GridColumn[]>(() => {
    return [
      { id: 'date', title: 'Date', width: 120 },
      { id: 'amount', title: 'Amount', width: 120 },
      { id: 'category', title: 'Category', width: 180 },
      { id: 'description', title: 'Description', width: 200, grow: 1 },
      { id: 'payer', title: 'Partner', width: 150 },
      { id: 'account', title: 'Account', width: 180 },
    ];
  }, []);

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

  const getCellContent = useCallback((cell: Item): GridCell => {
    const [col, rowIdx] = cell;
    const row = rows[rowIdx];

    if (!row) {
      return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
    }

    switch (col) {
      case 0:
        return {
          kind: GridCellKind.Text,
          data: row.date,
          displayData: formatDate(row.date),
          allowOverlay: true,
          twocentsEditor: 'date',
        } as TwocentsGridCell;
      case 1:
        return {
          kind: GridCellKind.Number,
          data: row.amount,
          displayData: row.amount != null ? formatCurrency(row.amount) : '',
          allowOverlay: true,
        };
      case 2: {
        const name = getCategoryName(row.category_id);
        return {
          kind: GridCellKind.Text,
          data: row.category_id,
          displayData: name,
          allowOverlay: true,
          twocentsEditor: 'category',
        } as TwocentsGridCell;
      }
      case 3:
        return {
          kind: GridCellKind.Text,
          data: row.description ?? '',
          displayData: row.description ?? '',
          allowOverlay: true,
        };
      case 4: {
        // Always get the member name if payer_id exists, otherwise show empty
        // Show placeholder if we have payer_id but can't find the name yet
        const displayName = row.payer_id ? getMemberName(row.payer_id) : '';
        const displayText = displayName || (row.payer_id ? 'Select partner...' : '');
        return {
          kind: GridCellKind.Text,
          data: row.payer_id || '',
          displayData: displayText, // Show placeholder instead of empty or UUID
          allowOverlay: true,
          twocentsEditor: 'payer',
        } as TwocentsGridCell;
      }
      case 5: {
        const name = getAccountName(row.account_id);
        return {
          kind: GridCellKind.Text,
          data: row.account_id || '',
          displayData: row.account_id ? name : '',
          allowOverlay: true,
          twocentsEditor: 'account',
        } as TwocentsGridCell;
      }
      default:
        return { kind: GridCellKind.Text, data: '', displayData: '', allowOverlay: false };
    }
  }, [rows, getCategoryName, getMemberName, getAccountName]);

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
        // Always provide editor, even if no options yet (will show empty dropdown)
        return (p: any) => {
          // Find the partner name from options using the UUID in data, or use displayData
          const payerId = p.value?.data || '';
          const partnerOption = partnerDropdownOptions.find(opt => opt.id === payerId);
          // Remove "Select partner..." placeholder if it's in displayData
          const displayData = p.value?.displayData || '';
          const cleanDisplayData = displayData === 'Select partner...' ? '' : displayData;
          const initialValue = partnerOption?.label || cleanDisplayData || '';
          return (
            <GridDropdownEditor
              {...p}
              initialValue={initialValue}
              title="Partner"
              options={partnerDropdownOptions}
              placeholder="Search partners…"
            />
          );
        };
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
    (col: number, row: QuickActionRow, newValue: EditableGridCell): QuickActionRow | null => {
      const getText = () => String((newValue as any).data ?? '').trim();

      switch (col) {
        case 0: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const nextDate = getText();
          if (!nextDate) return null;
          return { ...row, date: nextDate };
        }
        case 1: {
          const raw = (newValue as any).data;
          const nextAmount = typeof raw === 'number' ? raw : Number(raw);
          if (!Number.isFinite(nextAmount)) return null;
          return { ...row, amount: nextAmount };
        }
        case 2: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const input = getText();
          if (!input) return null;
          const match =
            categories.find((c) => c.id === input) ??
            categories.find((c) => c.name.toLowerCase() === input.toLowerCase());
          if (!match) return null;
          return { ...row, category_id: match.id };
        }
        case 3: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const input = String((newValue as any).data ?? '');
          return { ...row, description: input.trim() === '' ? '' : input };
        }
        case 4: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const raw = getText();
          if (!raw) return null;
          const match = householdMembers.find((m) => {
            const label = (m.profiles?.name || m.profiles?.email || '').toLowerCase();
            return m.user_id === raw || label === raw.toLowerCase();
          });
          if (!match) return null;
          return { ...row, payer_id: match.user_id };
        }
        case 5: {
          if (newValue.kind !== GridCellKind.Text) return null;
          const raw = getText();
          if (!raw) return null; // Account is required, don't allow empty
          const match =
            accounts.find((a) => a.id === raw) ??
            accounts.find((a) => a.name.toLowerCase() === raw.toLowerCase() || `${a.name} (${a.type})`.toLowerCase() === raw.toLowerCase());
          if (!match) return null;
          return { ...row, account_id: match.id };
        }
        default:
          return null;
      }
    },
    [accounts, categories, householdMembers]
  );

  const handleCellsEdited = useCallback(
    (items: readonly EditListItem[]) => {
      if (items.length === 0) return true;

      const rowPatches: Array<{ rowIdx: number; nextRow: QuickActionRow }> = [];

      for (const item of items) {
        const [col, rowIdx] = item.location;
        const row = rows[rowIdx];
        if (!row) continue;

        const computed = computeUpdateForCell(col, row, item.value);
        if (!computed) continue;

        rowPatches.push({ rowIdx, nextRow: computed });
      }

      if (rowPatches.length === 0) return true;

      scheduleMicrotask(() => {
        setRows((prevRows) => {
          const next = [...prevRows];
          for (const p of rowPatches) {
            if (p.rowIdx >= 0 && p.rowIdx < next.length) {
              next[p.rowIdx] = p.nextRow;
            }
          }
          return next;
        });
      });

      return true;
    },
    [rows, computeUpdateForCell]
  );

  const handleAddRow = useCallback(() => {
    const today = new Date().toISOString().split('T')[0];
    setRows((prev) => [
      ...prev,
      {
        id: generateRowId(),
        date: today,
        amount: null,
        category_id: '',
        description: '',
        payer_id: defaultPayerId || '',
        account_id: null,
      },
    ]);
  }, [defaultPayerId]);

  // Update rows when defaultPayerId changes (to prefill rows that don't have a partner)
  useEffect(() => {
    if (defaultPayerId) {
      setRows((prev) => 
        prev.map((row) => 
          row.payer_id ? row : { ...row, payer_id: defaultPayerId }
        )
      );
    }
  }, [defaultPayerId]);

  const validRows = useMemo(() => {
    return rows.filter(
      (row) => 
        row.date && 
        row.amount != null && 
        row.amount > 0 && 
        row.category_id && 
        row.payer_id && 
        row.account_id
    );
  }, [rows]);

  const invalidRows = useMemo(() => {
    return rows.map((row, idx) => {
      const missing: string[] = [];
      if (!row.date) missing.push('date');
      if (row.amount == null || row.amount <= 0) missing.push('amount');
      if (!row.category_id) missing.push('category');
      if (!row.payer_id) missing.push('partner');
      if (!row.account_id) missing.push('account');
      return { rowIdx: idx, missing };
    }).filter(r => r.missing.length > 0);
  }, [rows]);

  const handleSave = useCallback(async () => {
    if (validRows.length === 0) return;

    setSubmitting(true);
    try {
      await onSave(
        validRows.map((row) => ({
          date: row.date,
          amount: row.amount!,
          category_id: row.category_id,
          description: row.description || undefined,
          payer_id: row.payer_id,
          account_id: row.account_id,
        }))
      );
    } catch (error) {
      console.error('Error saving expenses:', error);
      throw error;
    } finally {
      setSubmitting(false);
    }
  }, [validRows, onSave]);

  return (
    <div className="flex flex-col gap-3">
      <div ref={gridRef} className="border border-border rounded-notion overflow-hidden">
        <DataEditor
          width={800}
          height={gridHeight}
          columns={glideColumns}
          rows={rows.length}
          getCellContent={getCellContent}
          onCellsEdited={handleCellsEdited}
          provideEditor={provideEditor}
          theme={glideTheme}
          getCellsForSelection={true}
          gridSelection={gridSelection}
          onGridSelectionChange={setGridSelection}
          cellActivationBehavior="single-click"
          editOnType={true}
          rowMarkers="none"
        />
      </div>
      {invalidRows.length > 0 && (
        <div className="rounded-notion border border-warning/50 bg-warning/10 p-3">
          <div className="text-sm font-medium text-warning-foreground mb-1">
            Missing required fields:
          </div>
          <div className="text-xs text-muted-foreground space-y-1">
            {invalidRows.map(({ rowIdx, missing }) => (
              <div key={rowIdx}>
                Row {rowIdx + 1}: Missing {missing.join(', ')}
              </div>
            ))}
          </div>
        </div>
      )}
      <div className="flex items-center justify-between">
        <button
          type="button"
          onClick={handleAddRow}
          className="text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          + Add Row
        </button>
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">
            {validRows.length} expense{validRows.length !== 1 ? 's' : ''} ready to save
            {invalidRows.length > 0 && ` (${invalidRows.length} incomplete)`}
          </span>
          <button
            type="button"
            onClick={handleSave}
            disabled={validRows.length === 0 || submitting}
            className="px-4 py-2 bg-accent text-accent-foreground rounded-notion hover:bg-accent/90 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium transition-colors"
          >
            {submitting ? 'Saving...' : `Save ${validRows.length} Expense${validRows.length !== 1 ? 's' : ''}`}
          </button>
        </div>
      </div>
    </div>
  );
}

