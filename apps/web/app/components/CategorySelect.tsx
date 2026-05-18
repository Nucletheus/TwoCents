'use client';

import { useMemo, forwardRef, useState, useRef, useEffect } from 'react';
import type { Category } from '@twocents/shared';
import { cn } from '@/lib/utils';
import { generateColorVariations, getCategoryColor } from '@twocents/shared';
import Input from './ui/Input';
import { ChevronDown, Search } from 'lucide-react';

interface CategorySelectProps {
  categories: Category[];
  value: string;
  onChange: (value: string) => void;
  required?: boolean;
  placeholder?: string;
  className?: string;
  onClick?: (e: React.MouseEvent) => void;
}

const CategorySelect = forwardRef<HTMLDivElement, CategorySelectProps>(
  function CategorySelect(
    {
      categories,
      value,
      onChange,
      required = false,
      placeholder = 'Select a category',
      className,
      onClick,
    },
    ref
  ) {
    const [isOpen, setIsOpen] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const [highlightedIndex, setHighlightedIndex] = useState(-1);
    const containerRef = useRef<HTMLDivElement>(null);
    const searchInputRef = useRef<HTMLInputElement>(null);
    const dropdownRef = useRef<HTMLDivElement>(null);

    // Get selected category
    const selectedCategory = categories.find((cat) => cat.id === value);

    // Group categories by group_name
    const groupedCategories = useMemo(() => {
      const groups: Record<string, Category[]> = {};
      const uncategorized: Category[] = [];

      categories.forEach((cat) => {
        if (cat.group_name) {
          if (!groups[cat.group_name]) {
            groups[cat.group_name] = [];
          }
          groups[cat.group_name].push(cat);
        } else {
          uncategorized.push(cat);
        }
      });

      // Sort groups alphabetically
      const sortedGroups = Object.keys(groups).sort();

      return { groups, sortedGroups, uncategorized };
    }, [categories]);

    // Filter categories based on search query
    const filteredGroups = useMemo(() => {
      if (!searchQuery.trim()) {
        return groupedCategories;
      }

      const query = searchQuery.toLowerCase();
      const filteredGroups: Record<string, Category[]> = {};
      const filteredUncategorized: Category[] = [];

      // Filter groups
      groupedCategories.sortedGroups.forEach((groupName) => {
        const filtered = groupedCategories.groups[groupName].filter(
          (cat) =>
            cat.name.toLowerCase().includes(query) ||
            (cat.group_name && cat.group_name.toLowerCase().includes(query))
        );
        if (filtered.length > 0) {
          filteredGroups[groupName] = filtered;
        }
      });

      // Filter uncategorized
      const filtered = groupedCategories.uncategorized.filter((cat) =>
        cat.name.toLowerCase().includes(query)
      );
      if (filtered.length > 0) {
        filteredUncategorized.push(...filtered);
      }

      return {
        groups: filteredGroups,
        sortedGroups: Object.keys(filteredGroups).sort(),
        uncategorized: filteredUncategorized,
      };
    }, [groupedCategories, searchQuery]);

    // Flatten categories for keyboard navigation
    const flatCategories = useMemo(() => {
      const flat: Category[] = [];
      filteredGroups.sortedGroups.forEach((groupName) => {
        flat.push(...filteredGroups.groups[groupName]);
      });
      flat.push(...filteredGroups.uncategorized);
      return flat;
    }, [filteredGroups]);

    // Close dropdown when clicking outside
    useEffect(() => {
      const handleClickOutside = (event: MouseEvent) => {
        if (
          containerRef.current &&
          !containerRef.current.contains(event.target as Node)
        ) {
          setIsOpen(false);
          setSearchQuery('');
          setHighlightedIndex(-1);
        }
      };

      if (isOpen) {
        document.addEventListener('mousedown', handleClickOutside);
        // Focus search input when dropdown opens
        setTimeout(() => {
          searchInputRef.current?.focus();
        }, 0);
      }

      return () => {
        document.removeEventListener('mousedown', handleClickOutside);
      };
    }, [isOpen]);

    // Handle keyboard navigation
    useEffect(() => {
      if (!isOpen) return;

      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          setIsOpen(false);
          setSearchQuery('');
          setHighlightedIndex(-1);
        } else if (e.key === 'ArrowDown') {
          e.preventDefault();
          setHighlightedIndex((prev) =>
            prev < flatCategories.length - 1 ? prev + 1 : prev
          );
        } else if (e.key === 'ArrowUp') {
          e.preventDefault();
          setHighlightedIndex((prev) => (prev > 0 ? prev - 1 : -1));
        } else if (e.key === 'Enter' && highlightedIndex >= 0) {
          e.preventDefault();
          const category = flatCategories[highlightedIndex];
          if (category) {
            onChange(category.id);
            setIsOpen(false);
            setSearchQuery('');
            setHighlightedIndex(-1);
          }
        }
      };

      window.addEventListener('keydown', handleKeyDown);
      return () => window.removeEventListener('keydown', handleKeyDown);
    }, [isOpen, flatCategories, highlightedIndex, onChange]);

    // Scroll highlighted item into view
    useEffect(() => {
      if (highlightedIndex >= 0 && dropdownRef.current) {
        const items = dropdownRef.current.querySelectorAll('[data-category-index]');
        const item = items[highlightedIndex] as HTMLElement;
        if (item) {
          item.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
        }
      }
    }, [highlightedIndex]);

    const handleSelect = (categoryId: string) => {
      onChange(categoryId);
      setIsOpen(false);
      setSearchQuery('');
      setHighlightedIndex(-1);
    };

    const handleButtonClick = (e: React.MouseEvent) => {
      e.stopPropagation();
      setIsOpen(!isOpen);
      onClick?.(e);
    };

    return (
      <div ref={containerRef} className={cn('relative', className)}>
        <button
          type="button"
          onClick={handleButtonClick}
          className={cn(
            'flex h-10 w-full items-center justify-between rounded-notion border border-border bg-background px-3 py-2 text-sm',
            'focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-0',
            'disabled:cursor-not-allowed disabled:opacity-50',
            isOpen && 'ring-2 ring-accent ring-offset-0',
            // Allow callers (e.g. compact table UIs) to size the trigger.
            // The `className` is still applied to the container above for layout purposes.
            className
          )}
          required={required}
        >
          <span className={cn('truncate', !selectedCategory && 'text-muted-foreground')}>
            {selectedCategory ? (
              <>
                {selectedCategory.icon && <span className="mr-2">{selectedCategory.icon}</span>}
                {selectedCategory.name}
              </>
            ) : (
              placeholder
            )}
          </span>
          <ChevronDown
            className={cn('h-4 w-4 flex-shrink-0 transition-transform', isOpen && 'rotate-180')}
          />
        </button>

        {isOpen && (
          <div
            ref={dropdownRef}
            className={cn(
              'z-50 mt-1 rounded-notion border border-border bg-background shadow-lg',
              'md:absolute md:max-h-[300px]',
              'fixed bottom-0 left-0 right-0 max-h-[70vh] md:relative md:bottom-auto md:left-auto md:right-auto'
            )}
            style={{ 
              width: '190px',
              overflow: 'visible', 
              display: 'flex', 
              flexDirection: 'column',
              backgroundColor: 'var(--background)',
              opacity: 1,
            }}
          >
            {/* Search input */}
            <div className="border-b border-border p-2">
              <div className="relative">
                <Search className="absolute left-2 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  ref={searchInputRef}
                  type="text"
                  placeholder="Search categories..."
                  value={searchQuery}
                  onChange={(e) => {
                    setSearchQuery(e.target.value);
                    setHighlightedIndex(-1);
                  }}
                  className="pl-8"
                  onClick={(e) => e.stopPropagation()}
                />
              </div>
            </div>

            {/* Categories list */}
            <div className="overflow-y-auto" style={{ maxHeight: '250px' }}>
              {flatCategories.length === 0 ? (
                <div className="px-3 py-2 text-sm text-muted-foreground">
                  No categories found
                </div>
              ) : (
                <>
                  {filteredGroups.sortedGroups.map((groupName) => {
                    const groupCategories = filteredGroups.groups[groupName];
                    const groupBaseColor =
                      groupCategories.find((c) => c.parent_color)?.parent_color ||
                      groupCategories.find((c) => c.color)?.color ||
                      null;
                    return (
                      <div key={groupName}>
                        <div 
                          className="text-xs font-medium uppercase tracking-wider text-muted-foreground"
                          style={{
                            height: '18px',
                            paddingTop: '2px',
                            paddingBottom: '2px',
                            paddingLeft: '7px',
                            paddingRight: '7px',
                            backgroundColor: 'rgba(209, 209, 209, 0.5)',
                          }}
                        >
                          {groupName}
                        </div>
                        {groupCategories.map((cat, idx) => {
                          const flatIndex = flatCategories.indexOf(cat);
                          const categoryColor = groupBaseColor
                            ? generateColorVariations(groupBaseColor, idx, groupCategories.length)
                            : getCategoryColor(cat, idx, groupCategories.length);
                          return (
                            <button
                              key={cat.id}
                              type="button"
                              data-category-index={flatIndex}
                              onClick={() => handleSelect(cat.id)}
                              className={cn(
                                'flex w-full items-center px-3 py-2 text-left text-[10px] transition-colors',
                                'hover:bg-hover',
                                cat.id === value && 'bg-accent/10',
                                highlightedIndex === flatIndex && 'bg-hover'
                              )}
                              style={{
                                borderLeft: `3px solid ${categoryColor}`,
                                height: '23px',
                                verticalAlign: 'middle',
                              }}
                            >
                              {cat.icon && <span className="mr-2">{cat.icon}</span>}
                              <span>{cat.name}</span>
                            </button>
                          );
                        })}
                      </div>
                    );
                  })}
                  {filteredGroups.uncategorized.length > 0 && (
                    <div>
                      <div 
                        className="text-xs font-medium uppercase tracking-wider text-muted-foreground"
                        style={{
                          height: '18px',
                          paddingTop: '2px',
                          paddingBottom: '2px',
                          paddingLeft: '7px',
                          paddingRight: '7px',
                          backgroundColor: 'rgba(209, 209, 209, 0.5)',
                        }}
                      >
                        Other
                      </div>
                      {filteredGroups.uncategorized.map((cat, idx) => {
                        const flatIndex = flatCategories.indexOf(cat);
                        const categoryColor = getCategoryColor(cat, idx, filteredGroups.uncategorized.length);
                        return (
                          <button
                            key={cat.id}
                            type="button"
                            data-category-index={flatIndex}
                            onClick={() => handleSelect(cat.id)}
                            className={cn(
                              'flex w-full items-center px-3 py-2 text-left text-[10px] transition-colors',
                              'hover:bg-hover',
                              cat.id === value && 'bg-accent/10',
                              highlightedIndex === flatIndex && 'bg-hover'
                            )}
                            style={{
                              borderLeft: `3px solid ${categoryColor}`,
                              height: '23px',
                              verticalAlign: 'middle',
                            }}
                          >
                            {cat.icon && <span className="mr-2">{cat.icon}</span>}
                            <span>{cat.name}</span>
                          </button>
                        );
                      })}
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        )}
      </div>
    );
  }
);

CategorySelect.displayName = 'CategorySelect';

export default CategorySelect;

