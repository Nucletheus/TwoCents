'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { Category } from '@twocents/shared';
import { generateColorVariations, getCategoryColor } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Input from '../components/ui/Input';
import Button from '../components/ui/Button';
import { ChevronDown, X, Search } from 'lucide-react';

type SplitMap = Record<string, Record<string, number>>;

export default function SharedCategoriesDialog({
  isOpen,
  onClose,
  categories,
  memberIds,
  memberNameMap,
  selectedCategoryIds,
  splitsByCategory,
  onToggleCategory,
  onSetCategoriesShared,
  onSplitChange,
  onSave,
  saving = false,
  error,
}: {
  isOpen: boolean;
  onClose: () => void;
  categories: Category[];
  memberIds: string[];
  memberNameMap: Map<string, string>;
  selectedCategoryIds: string[];
  splitsByCategory: SplitMap;
  onToggleCategory: (categoryId: string) => void;
  onSetCategoriesShared?: (categoryIds: string[], shared: boolean) => void;
  onSplitChange: (categoryId: string, memberId: string, value: string) => void;
  onSave: () => void;
  saving?: boolean;
  error?: string | null;
}) {
  const [searchQuery, setSearchQuery] = useState('');
  const [mounted, setMounted] = useState(false);
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>({});

  useEffect(() => {
    setMounted(true);
  }, []);

  const selectedIdSet = useMemo(() => new Set(selectedCategoryIds), [selectedCategoryIds]);

  const setCategoriesShared = (categoryIds: string[], shared: boolean) => {
    if (categoryIds.length === 0) return;
    if (onSetCategoriesShared) {
      onSetCategoriesShared(categoryIds, shared);
      return;
    }
    categoryIds.forEach((categoryId) => {
      const isSelected = selectedIdSet.has(categoryId);
      if (shared && !isSelected) onToggleCategory(categoryId);
      if (!shared && isSelected) onToggleCategory(categoryId);
    });
  };

  const selectedChips = useMemo(() => {
    if (selectedCategoryIds.length === 0) return [];

    type Chip = { key: string; label: string; icon: string | null; color: string | null; ids: string[]; count?: number };

    const byId = new Map(categories.map((c) => [c.id, c]));
    const selectedSet = new Set(selectedCategoryIds);

    // Build group membership
    const groupedByName = new Map<string, Category[]>();
    const ungrouped: Category[] = [];
    const seenIds = new Set<string>();

    categories.forEach((cat) => {
      if (seenIds.has(cat.id)) return;
      seenIds.add(cat.id);

      if (cat.group_name) {
        const list = groupedByName.get(cat.group_name) || [];
        list.push(cat);
        groupedByName.set(cat.group_name, list);
      } else {
        ungrouped.push(cat);
      }
    });

    const groupNames = Array.from(groupedByName.keys()).sort((a, b) => a.localeCompare(b));

    const fullySelectedGroups = new Set<string>();
    groupNames.forEach((groupName) => {
      const ids = (groupedByName.get(groupName) || []).map((c) => c.id);
      if (ids.length > 0 && ids.every((id) => selectedSet.has(id))) {
        fullySelectedGroups.add(groupName);
      }
    });
    if (ungrouped.length > 0) {
      const ids = ungrouped.map((c) => c.id);
      if (ids.length > 0 && ids.every((id) => selectedSet.has(id))) {
        fullySelectedGroups.add('Other');
      }
    }

    const groupChips: Chip[] = [];
    groupNames.forEach((groupName) => {
      if (!fullySelectedGroups.has(groupName)) return;
      const cats = groupedByName.get(groupName) || [];
      const ids = cats.map((c) => c.id).sort();
      const groupBaseColor = cats.find((c) => c.parent_color)?.parent_color || cats.find((c) => c.color)?.color || null;
      groupChips.push({
        key: `group:${groupName}`,
        label: groupName,
        icon: null,
        color: groupBaseColor,
        ids,
        count: cats.length,
      });
    });

    if (fullySelectedGroups.has('Other')) {
      const ids = ungrouped.map((c) => c.id).sort();
      const groupBaseColor =
        ungrouped.find((c) => c.parent_color)?.parent_color || ungrouped.find((c) => c.color)?.color || null;
      groupChips.push({
        key: 'group:Other',
        label: 'Other',
        icon: null,
        color: groupBaseColor,
        ids,
        count: ungrouped.length,
      });
    }

    const chipsByCategory = new Map<string, Chip>();
    selectedCategoryIds.forEach((categoryId) => {
      const cat = byId.get(categoryId);
      if (!cat) return;
      const groupName = cat.group_name || 'Other';
      if (fullySelectedGroups.has(groupName)) return;

      const key = `cat:${groupName}|${cat.name.trim().toLowerCase()}`;
      const existing = chipsByCategory.get(key);
      if (existing) {
        existing.ids.push(categoryId);
        return;
      }
      chipsByCategory.set(key, {
        key,
        label: cat.name,
        icon: cat.icon,
        color: cat.parent_color || cat.color || null,
        ids: [categoryId],
      });
    });

    const categoryChips = Array.from(chipsByCategory.values()).map((chip) => ({
      ...chip,
      ids: chip.ids.sort(),
    }));
    categoryChips.sort((a, b) => a.label.localeCompare(b.label));

    return [...groupChips, ...categoryChips];
  }, [categories, selectedCategoryIds]);

  const grouped = useMemo(() => {
    const byGroup: Record<string, Category[]> = {};
    const ungrouped: Category[] = [];
    const seenIds = new Set<string>();

    categories.forEach((cat) => {
      if (seenIds.has(cat.id)) return;
      seenIds.add(cat.id);

      if (cat.group_name) {
        if (!byGroup[cat.group_name]) byGroup[cat.group_name] = [];
        byGroup[cat.group_name].push(cat);
      } else {
        ungrouped.push(cat);
      }
    });

    const groupNames = Object.keys(byGroup).sort((a, b) => a.localeCompare(b));
    groupNames.forEach((g) => byGroup[g].sort((a, b) => a.name.localeCompare(b.name)));
    ungrouped.sort((a, b) => a.name.localeCompare(b.name));

    // Safety: ensure ungrouped never includes a category already present in any group.
    const groupedIds = new Set<string>();
    groupNames.forEach((g) => byGroup[g].forEach((cat) => groupedIds.add(cat.id)));
    const dedupedUngrouped = ungrouped.filter((cat) => !groupedIds.has(cat.id));

    return { byGroup, groupNames, ungrouped: dedupedUngrouped };
  }, [categories]);

  const filtered = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return grouped;

    const byGroup: Record<string, Category[]> = {};
    const ungrouped: Category[] = [];

    grouped.groupNames.forEach((groupName) => {
      const hits = grouped.byGroup[groupName].filter(
        (cat) => cat.name.toLowerCase().includes(q) || groupName.toLowerCase().includes(q)
      );
      if (hits.length > 0) byGroup[groupName] = hits;
    });

    grouped.ungrouped.forEach((cat) => {
      if (cat.name.toLowerCase().includes(q)) ungrouped.push(cat);
    });

    return { byGroup, groupNames: Object.keys(byGroup).sort((a, b) => a.localeCompare(b)), ungrouped };
  }, [grouped, searchQuery]);

  const colCount = 2 + memberIds.length;
  const allowTwoColumnLayout = memberIds.length <= 2;

  const groupBlocks = useMemo(() => {
    const blocks: Array<{ key: string; groupName: string; categories: Category[] }> = [];
    const ungrouped = filtered.ungrouped || [];

    // If the dataset already has an explicit group called "Other", merge ungrouped into it
    // so we never render two separate "Other" sections.
    const otherGroupName =
      filtered.groupNames.find((g) => g.trim().toLowerCase() === 'other') || null;

    const mergeUniqueById = (a: Category[], b: Category[]) => {
      const seen = new Set<string>();
      const out: Category[] = [];
      [...a, ...b].forEach((cat) => {
        if (seen.has(cat.id)) return;
        seen.add(cat.id);
        out.push(cat);
      });
      return out;
    };

    filtered.groupNames.forEach((groupName) => {
      const cats = filtered.byGroup[groupName] || [];
      if (cats.length === 0 && !(otherGroupName === groupName && ungrouped.length > 0)) return;

      const mergedCats =
        otherGroupName === groupName && ungrouped.length > 0 ? mergeUniqueById(cats, ungrouped) : cats;

      blocks.push({ key: `group:${groupName}`, groupName, categories: mergedCats });
    });

    // If there isn't an explicit "Other" group, create a single ungrouped "Other" block.
    if (!otherGroupName && ungrouped.length > 0) {
      blocks.push({ key: 'group:__ungrouped__', groupName: 'Other', categories: ungrouped });
    }

    return blocks;
  }, [filtered]);

  useEffect(() => {
    if (!isOpen) return;
    // Start with all groups collapsed by default (but don't override user-toggled state).
    setCollapsedGroups((prev) => {
      const next = { ...prev };
      groupBlocks.forEach((block) => {
        if (next[block.groupName] === undefined) {
          next[block.groupName] = true;
        }
      });
      return next;
    });
  }, [isOpen, groupBlocks]);

  const tableColumns = useMemo(() => {
    if (!allowTwoColumnLayout) return [groupBlocks];
    if (groupBlocks.length <= 1) return [groupBlocks];

    const left: typeof groupBlocks = [];
    const right: typeof groupBlocks = [];
    let leftCount = 0;
    let rightCount = 0;

    groupBlocks.forEach((block) => {
      const blockRows = (block.categories?.length || 0) + 1; // group header + rows
      if (leftCount <= rightCount) {
        left.push(block);
        leftCount += blockRows;
      } else {
        right.push(block);
        rightCount += blockRows;
      }
    });

    const cols = [left, right].filter((c) => c.length > 0);
    return cols.length > 0 ? cols : [groupBlocks];
  }, [allowTwoColumnLayout, groupBlocks]);

  if (!isOpen || !mounted) return null;

  const content = (
    <div
      className="fixed inset-0 z-[80] flex items-center justify-center bg-black/50 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <Card className="w-full max-w-6xl max-h-[95vh] overflow-hidden flex flex-col">
        <CardHeader className="flex-shrink-0 p-4 pb-3">
          <div className="flex items-center justify-between gap-3">
            <div className="min-w-0">
              <CardTitle>Shared categories</CardTitle>
              <CardDescription>Choose shared categories and adjust how they’re split</CardDescription>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="rounded p-1 hover:bg-hover transition-colors"
              aria-label="Close"
            >
              <X className="h-5 w-5" />
            </button>
          </div>

          <div className="mt-2 flex flex-wrap items-center gap-2">
            <div className="relative flex-1 min-w-[220px]">
              <Search className="absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search categories..."
                className="h-9 px-3 py-[9px] pl-10 text-sm justify-center items-center gap-0 ml-[1px] mr-[1px]"
              />
            </div>
          </div>
        </CardHeader>

        <CardContent className="p-4 pt-0 flex flex-col gap-3">
          {error && (
            <div className="rounded-notion border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
              {error}
            </div>
          )}

          {selectedChips.length > 0 && (
            <div className="rounded-notion border border-border bg-card/40 px-2 py-2">
              <div className="flex items-center justify-between gap-2">
                <span className="text-[11px] font-medium text-muted-foreground uppercase tracking-wider">Selected</span>
                <span className="text-xs text-muted-foreground">{selectedChips.length}</span>
              </div>
              <div className="mt-2 flex gap-1 overflow-x-auto scrollbar-hide">
                {selectedChips.map((chip) => (
                  <span
                    key={chip.key}
                    className="inline-flex items-center gap-1 rounded-notion border border-border bg-background px-2 py-1 text-xs"
                  >
                    {chip.color && <span className="h-3 w-1 rounded-full" style={{ backgroundColor: chip.color }} />}
                    {chip.icon && <span className="text-sm">{chip.icon}</span>}
                    <span className="max-w-[160px] truncate">{chip.label}</span>
                    {typeof chip.count === 'number' && (
                      <span className="text-[10px] text-muted-foreground tabular-nums">[{chip.count}]</span>
                    )}
                    <button
                      type="button"
                      onClick={() => setCategoriesShared(chip.ids, false)}
                      className="ml-1 rounded p-0.5 hover:bg-hover"
                      aria-label={`Remove ${chip.label}`}
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </span>
                ))}
              </div>
            </div>
          )}

          <div className="rounded-notion border border-border">
            {groupBlocks.length === 0 ? (
              <div className="px-3 py-8 text-center text-sm text-muted-foreground">No categories match your search.</div>
            ) : (
              <div
                className={`grid grid-cols-1 ${
                  allowTwoColumnLayout && tableColumns.length > 1
                    ? 'md:grid-cols-2 md:divide-x md:divide-border/60'
                    : ''
                }`}
              >
                {tableColumns.map((blocks, idx) => (
                  <div key={idx} className="overflow-y-auto max-h-[calc(95vh-300px)]">
                    <table className="w-full border-collapse">
                      <thead className="sticky top-0 z-10 bg-card border-b border-border">
                        <tr className="text-left">
                          <th className="w-16 px-2 py-1 text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
                            Shared
                          </th>
                          <th className="min-w-[220px] px-2 py-1 text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
                            Category
                          </th>
                          {memberIds.map((memberId) => (
                            <th
                              key={memberId}
                              className="min-w-[140px] px-2 py-1 text-[11px] font-medium text-muted-foreground uppercase tracking-wider"
                            >
                              {memberNameMap.get(memberId) || 'Member'}
                            </th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {blocks.map((block) => (
                          <FragmentedGroup
                            key={`${idx}-${block.key}`}
                            groupName={block.groupName}
                            categories={block.categories}
                            memberIds={memberIds}
                            memberNameMap={memberNameMap}
                            selectedCategoryIds={selectedCategoryIds}
                            selectedIdSet={selectedIdSet}
                            splitsByCategory={splitsByCategory}
                            onToggleCategory={onToggleCategory}
                            onSetCategoriesShared={setCategoriesShared}
                            onSplitChange={onSplitChange}
                            colCount={colCount}
                            collapsed={!!collapsedGroups[block.groupName]}
                            onToggleCollapsed={() =>
                              setCollapsedGroups((prev) => ({
                                ...prev,
                                [block.groupName]: !prev[block.groupName],
                              }))
                            }
                          />
                        ))}
                      </tbody>
                    </table>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="flex items-center justify-end gap-2 border-t border-border pt-0">
            <Button type="button" variant="ghost" size="sm" onClick={onClose} disabled={saving}>
              Cancel
            </Button>
            <Button type="button" size="sm" onClick={onSave} disabled={saving || memberIds.length === 0}>
              {saving ? 'Saving...' : 'Save'}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );

  return createPortal(content, document.body);
}

function FragmentedGroup({
  groupName,
  categories,
  memberIds,
  memberNameMap,
  selectedCategoryIds,
  selectedIdSet,
  splitsByCategory,
  onToggleCategory,
  onSetCategoriesShared,
  onSplitChange,
  colCount,
  collapsed,
  onToggleCollapsed,
}: {
  groupName: string;
  categories: Category[];
  memberIds: string[];
  memberNameMap: Map<string, string>;
  selectedCategoryIds: string[];
  selectedIdSet: Set<string>;
  splitsByCategory: SplitMap;
  onToggleCategory: (categoryId: string) => void;
  onSetCategoriesShared: (categoryIds: string[], shared: boolean) => void;
  onSplitChange: (categoryId: string, memberId: string, value: string) => void;
  colCount: number;
  collapsed: boolean;
  onToggleCollapsed: () => void;
}) {
  const rows = useMemo(() => {
    if (groupName !== 'Other') {
      return categories.map((cat) => ({
        key: cat.id,
        category: cat,
        ids: [cat.id],
      }));
    }

    // "Other" can contain default + household categories with the same name.
    // Merge them into a single row so users don't see duplicates.
    const byName = new Map<string, Category[]>();
    categories.forEach((cat) => {
      const nameKey = cat.name.trim().toLowerCase();
      const list = byName.get(nameKey) || [];
      list.push(cat);
      byName.set(nameKey, list);
    });

    const merged = Array.from(byName.entries()).map(([nameKey, list]) => {
      const preferred = list.find((c) => c.household_id) || list[0]!;
      return {
        key: `other-${nameKey}`,
        category: preferred,
        ids: list.map((c) => c.id),
      };
    });

    merged.sort((a, b) => a.category.name.localeCompare(b.category.name));
    return merged;
  }, [categories, groupName]);

  const allIds = useMemo(() => rows.flatMap((r) => r.ids), [rows]);
  const selectedRowsCount = useMemo(
    () => rows.filter((r) => r.ids.every((id) => selectedIdSet.has(id))).length,
    [rows, selectedIdSet]
  );
  const allSelected = allIds.length > 0 && allIds.every((id) => selectedIdSet.has(id));
  const someSelected = allIds.some((id) => selectedIdSet.has(id)) && !allSelected;

  const groupCheckboxRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (groupCheckboxRef.current) {
      groupCheckboxRef.current.indeterminate = someSelected;
    }
  }, [someSelected]);

  const groupBaseColor =
    rows.find((r) => r.category.parent_color)?.category.parent_color ||
    rows.find((r) => r.category.color)?.category.color ||
    null;
  const groupHeaderStyle = groupBaseColor
    ? {
        backgroundColor: hexToRgba(groupBaseColor, 0.08),
      }
    : undefined;

  return (
    <>
      <tr>
        <td
          colSpan={colCount}
          className="px-2 py-1 text-[11px] font-medium text-muted-foreground uppercase tracking-wider"
          style={groupHeaderStyle}
        >
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onToggleCollapsed}
                className="rounded p-0.5 hover:bg-hover transition-colors"
                aria-label={collapsed ? `Expand ${groupName}` : `Collapse ${groupName}`}
              >
                <ChevronDown className={`h-4 w-4 transition-transform ${collapsed ? '-rotate-90' : ''}`} />
              </button>
              <input
                ref={groupCheckboxRef}
                type="checkbox"
                checked={allSelected}
                onChange={() => onSetCategoriesShared(allIds, !allSelected)}
                className="h-4 w-4"
                aria-label={`Select all ${groupName}`}
              />
              <span
                className="h-4 w-1.5 rounded-full"
                style={{ backgroundColor: groupBaseColor || 'transparent' }}
              />
              <span>{groupName}</span>
              <span className="text-[11px] text-muted-foreground tabular-nums">
                {selectedRowsCount}/{rows.length}
              </span>
            </div>
          </div>
        </td>
      </tr>

      {!collapsed &&
        rows.map((row, idx) => {
          const { category, ids } = row;
          const selectedCount = ids.filter((id) => selectedIdSet.has(id)).length;
          const rowAllSelected = selectedCount === ids.length && ids.length > 0;
          const rowSomeSelected = selectedCount > 0 && !rowAllSelected;

          const splitSourceId = ids.find((id) => splitsByCategory[id]) || ids[0];
          const split = (splitSourceId && splitsByCategory[splitSourceId]) || {};

          const rowColor = groupBaseColor
            ? generateColorVariations(groupBaseColor, idx, rows.length)
            : getCategoryColor(category, idx, rows.length);

        return (
          <tr key={row.key} className="border-b border-border hover:bg-hover">
            <td className="px-2 py-0.5">
              <input
                type="checkbox"
                checked={rowAllSelected}
                ref={(el) => {
                  if (el) el.indeterminate = rowSomeSelected;
                }}
                onChange={() => onSetCategoriesShared(ids, !rowAllSelected)}
                className="h-4 w-4"
              />
            </td>
            <td className="px-2 py-0.5">
              <div className="flex items-center gap-2">
                <span className="h-4 w-1.5 rounded-full" style={{ backgroundColor: rowColor }} />
                {category.icon && <span className="text-base">{category.icon}</span>}
                <span className="text-sm font-medium">{category.name}</span>
              </div>
            </td>
            {memberIds.map((memberId) => {
              const value = rowAllSelected ? split?.[memberId] ?? 0 : '';
              const label = memberNameMap.get(memberId) || 'Member';
              return (
                <td key={memberId} className="px-2 py-0.5">
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      min={0}
                      max={100}
                      step="0.1"
                      value={value as any}
                      onChange={(e) => {
                        ids.forEach((id) => onSplitChange(id, memberId, e.target.value));
                      }}
                      disabled={!rowAllSelected || memberIds.length === 1}
                      aria-label={`${label} split for ${category.name}`}
                      className="h-8 w-20 px-2 text-sm"
                    />
                    <span className="text-[11px] text-muted-foreground">%</span>
                  </div>
                </td>
              );
            })}
          </tr>
        );
        })}
    </>
  );
}

function hexToRgba(hex: string, alpha: number): string {
  const clean = hex.replace('#', '').trim();
  if (clean.length !== 6) return `rgba(0,0,0,${alpha})`;
  const r = parseInt(clean.slice(0, 2), 16);
  const g = parseInt(clean.slice(2, 4), 16);
  const b = parseInt(clean.slice(4, 6), 16);
  if ([r, g, b].some((v) => Number.isNaN(v))) return `rgba(0,0,0,${alpha})`;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}


