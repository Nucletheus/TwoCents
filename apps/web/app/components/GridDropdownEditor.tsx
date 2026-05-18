'use client';

import { useState, useCallback, useMemo, useEffect, useRef } from 'react';
import Input from './ui/Input';
import { GridCellKind } from '@glideapps/glide-data-grid';

export type DropdownOption = {
  id: string;
  label: string;
  subLabel?: string;
  group?: string;
  color?: string;
  parentColor?: string;
  icon?: string | null;
};

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

export default function GridDropdownEditor({
  value,
  onFinishedEditing,
  initialValue,
  title,
  options,
  placeholder,
}: {
  value: any;
  onFinishedEditing: (newValue?: any, movement?: readonly [0 | 1 | -1, 0 | 1 | -1]) => void;
  initialValue?: string;
  title: string;
  options: DropdownOption[];
  placeholder?: string;
}) {
  const [query, setQuery] = useState<string>(initialValue ?? '');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Always focus the input first when dropdown opens
    inputRef.current?.focus({ preventScroll: true });
    if (query) {
      inputRef.current?.setSelectionRange(query.length, query.length);
    }
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
    const groupColors: Record<string, string> = {};
    const ungrouped: DropdownOption[] = [];
    for (const opt of filtered) {
      if (opt.group) {
        if (!groups[opt.group]) groups[opt.group] = [];
        groups[opt.group].push(opt);
        // Use parentColor if available, otherwise use the first color found
        if (!groupColors[opt.group]) {
          groupColors[opt.group] = opt.parentColor || opt.color || '';
        }
      } else {
        ungrouped.push(opt);
      }
    }
    const sortedGroups = Object.keys(groups).sort((a, b) => a.localeCompare(b));
    return { groups, sortedGroups, ungrouped, groupColors };
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

  // Reset highlight when filtered results change (due to query or options change)
  useEffect(() => {
    if (flat.length === 0) {
      setHighlighted(-1);
    } else {
      // Reset to first item when results change
      setHighlighted(0);
    }
  }, [flat.length, query]); // Reset when query changes to highlight first filtered result

  // Auto-scroll highlighted item into view
  useEffect(() => {
    if (highlighted >= 0 && highlightedItemRef.current) {
      highlightedItemRef.current.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest',
      });
    }
  }, [highlighted]);

  const commit = useCallback((id: string) => {
    const opt = options.find((o) => o.id === id);
    commitTextCell(value, id, opt?.label ?? id, onFinishedEditing);
  }, [options, value, onFinishedEditing]);

  // Keyboard navigation handler (shared between input and list)
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    // Handle navigation keys that should select items, not scroll
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter' || 
        e.key === 'Home' || e.key === 'End' || e.key === 'PageDown' || 
        e.key === 'PageUp' || e.key === 'Escape') {
      
      if (flat.length === 0) {
        if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          onFinishedEditing(undefined);
        }
        return;
      }

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        e.stopPropagation();
        setHighlighted((h) => {
          if (h < 0) return 0;
          return Math.min(flat.length - 1, h + 1);
        });
        return;
      }
      
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        e.stopPropagation();
        setHighlighted((h) => {
          // If at the top (0 or less), move focus back to input
          if (h <= 0) {
            inputRef.current?.focus({ preventScroll: true });
            return -1; // Clear highlight
          }
          return Math.max(0, h - 1);
        });
        return;
      }
      
      if (e.key === 'Enter') {
        e.preventDefault();
        e.stopPropagation();
        if (highlighted >= 0 && highlighted < flat.length) {
          const opt = flat[highlighted];
          if (opt) {
            commit(opt.id);
          }
        }
        return;
      }
      
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onFinishedEditing(undefined);
        return;
      }
      
      if (e.key === 'Home') {
        e.preventDefault();
        e.stopPropagation();
        setHighlighted(0);
        return;
      }
      
      if (e.key === 'End') {
        e.preventDefault();
        e.stopPropagation();
        setHighlighted(flat.length - 1);
        return;
      }
      
      if (e.key === 'PageDown') {
        e.preventDefault();
        e.stopPropagation();
        setHighlighted((h) => {
          const current = h < 0 ? 0 : h;
          return Math.min(flat.length - 1, current + 10);
        });
        return;
      }
      
      if (e.key === 'PageUp') {
        e.preventDefault();
        e.stopPropagation();
        setHighlighted((h) => {
          const current = h < 0 ? 0 : h;
          return Math.max(0, current - 10);
        });
        return;
      }
    }
  }, [flat, highlighted, commit, onFinishedEditing]);

  return (
    <div className="min-w-[320px] max-w-[420px] pl-1 pr-1 overflow-hidden flex flex-col">
      <div className="mb-2 text-xs font-medium text-muted-foreground flex-shrink-0">{title}</div>
      <Input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={placeholder ?? 'Search…'}
        className="h-9 text-sm px-1 justify-center items-center w-[310px] flex-shrink-0"
        onKeyDown={(e) => {
          // When user presses ArrowDown, move focus to list and activate navigation
          if (e.key === 'ArrowDown') {
            e.preventDefault();
            e.stopPropagation();
            listRef.current?.focus({ preventScroll: true });
            // Trigger the navigation handler
            handleKeyDown(e);
            return;
          }
          
          // Handle other navigation keys only if list is focused or after ArrowDown
          if (e.key === 'ArrowUp' || e.key === 'Enter' || 
              e.key === 'Home' || e.key === 'End' || e.key === 'PageDown' || 
              e.key === 'PageUp' || e.key === 'Escape') {
            // For ArrowUp, focus list first
            if (e.key === 'ArrowUp') {
              listRef.current?.focus({ preventScroll: true });
            }
            handleKeyDown(e);
          }
        }}
      />

      <div 
        ref={listRef} 
        tabIndex={0}
        role="listbox"
        aria-label="Category options"
        aria-activedescendant={highlighted >= 0 && highlighted < flat.length ? `option-${flat[highlighted]?.id}` : undefined}
        className="mt-2 max-h-[280px] overflow-y-auto overflow-x-hidden border border-border rounded-notion flex-shrink focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-1"
        onKeyDown={handleKeyDown}
        onFocus={() => {
          // When list gets focus, ensure an item is highlighted
          if (highlighted < 0 && flat.length > 0) {
            setHighlighted(0);
          }
        }}
        onMouseDown={(e) => {
          // Prevent input from losing focus when clicking on list, but allow list to receive focus
          listRef.current?.focus();
        }}
      >
        {grouped.sortedGroups.length > 0 && (
          <>
            {grouped.sortedGroups.map((groupName) => {
              const groupColor = grouped.groupColors[groupName];
              return (
                <div key={groupName}>
                  <div
                    className="sticky top-0 px-2 py-1 text-[10px] font-medium uppercase tracking-wide"
                    style={{
                      background: groupColor
                        ? `linear-gradient(90deg, ${groupColor} 0%, rgba(255, 255, 255, 1) 100%)`
                        : 'var(--muted)',
                      color: groupColor ? 'var(--foreground)' : 'var(--muted-foreground)',
                    }}
                  >
                    {groupName}
                  </div>
                  {grouped.groups[groupName].map((opt, idx) => {
                    const flatIdx = flat.indexOf(opt);
                    const isHighlighted = flatIdx === highlighted;
                    return (
                      <button
                        key={opt.id}
                        id={`option-${opt.id}`}
                        ref={isHighlighted ? highlightedItemRef : null}
                        type="button"
                        role="option"
                        aria-selected={isHighlighted}
                        className={[
                          'w-full px-2 py-1.5 text-left text-xs flex items-center gap-2 hover:bg-hover transition-colors',
                          isHighlighted ? 'bg-accent/30 ring-2 ring-accent ring-offset-1 font-medium' : '',
                        ].join(' ')}
                        style={{
                          borderLeft: opt.color ? `3px solid ${opt.color}` : undefined,
                        }}
                        onClick={() => commit(opt.id)}
                        onMouseEnter={() => setHighlighted(flatIdx)}
                      >
                        {opt.icon && <span className="text-base">{opt.icon}</span>}
                        <div className="flex-1 min-w-0">
                          <div className="truncate">{opt.label}</div>
                          {opt.subLabel && (
                            <div className="text-[10px] text-muted-foreground truncate">{opt.subLabel}</div>
                          )}
                        </div>
                      </button>
                    );
                  })}
                </div>
              );
            })}
          </>
        )}
        {grouped.ungrouped.length > 0 && (
          <div>
            {grouped.sortedGroups.length > 0 && (
              <div className="sticky top-0 bg-muted/50 px-2 py-1 text-[10px] font-medium text-muted-foreground uppercase tracking-wide">
                Other
              </div>
            )}
            {grouped.ungrouped.map((opt, idx) => {
              const flatIdx = flat.indexOf(opt);
              const isHighlighted = flatIdx === highlighted;
              return (
                <button
                  key={opt.id}
                  id={`option-${opt.id}`}
                  ref={isHighlighted ? highlightedItemRef : null}
                  type="button"
                  role="option"
                  aria-selected={isHighlighted}
                  className={[
                    'w-full px-2 py-1.5 text-left text-xs flex items-center gap-2 hover:bg-hover transition-colors',
                    isHighlighted ? 'bg-accent/30 ring-2 ring-accent ring-offset-1 font-medium' : '',
                  ].join(' ')}
                  style={{
                    borderLeft: opt.color ? `3px solid ${opt.color}` : undefined,
                  }}
                  onClick={() => commit(opt.id)}
                  onMouseEnter={() => setHighlighted(flatIdx)}
                >
                  {opt.icon && <span className="text-base">{opt.icon}</span>}
                  <div className="flex-1 min-w-0">
                    <div className="truncate">{opt.label}</div>
                    {opt.subLabel && (
                      <div className="text-[10px] text-muted-foreground truncate">{opt.subLabel}</div>
                    )}
                  </div>
                </button>
              );
            })}
          </div>
        )}
        {flat.length === 0 && (
          <div className="px-2 py-4 text-center text-xs text-muted-foreground">No matches</div>
        )}
      </div>
    </div>
  );
}

