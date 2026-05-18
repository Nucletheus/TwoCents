'use client';

import { useState, useEffect, useMemo, ReactNode, forwardRef, useRef } from 'react';
import { cn } from '@/lib/utils';
import { ChevronDown, Check } from 'lucide-react';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from './command';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from './popover';

export interface SearchableSelectItem {
  id: string;
  label: string;
  subLabel?: string;
  group?: string;
  icon?: ReactNode;
  color?: string;
  [key: string]: any;
}

interface SearchableSelectProps {
  items: SearchableSelectItem[];
  value: string | null | undefined;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  searchPlaceholder?: string;
  required?: boolean;
  disabled?: boolean;
  renderItem?: (item: SearchableSelectItem) => ReactNode;
  smartSuggestions?: string[]; // Array of IDs to show at the top
  defaultOpen?: boolean; // Open dropdown on mount
  onClose?: () => void; // Callback when dropdown closes without selection
}

const SearchableSelect = forwardRef<HTMLDivElement, SearchableSelectProps>(
  function SearchableSelect(
    {
      items,
      value,
      onChange,
      placeholder = 'Select option...',
      className,
      searchPlaceholder = 'Search...',
      required = false,
      disabled = false,
      renderItem,
      smartSuggestions = [],
      defaultOpen = false,
      onClose,
    },
    ref
  ) {
    const [open, setOpen] = useState(defaultOpen);
    const selectedItem = useMemo(() => items.find((item) => item.id === value), [items, value]);
    const justOpenedRef = useRef(false);

    // Process items: Filter -> Group -> Sort (for smart suggestions)
    const processedItems = useMemo(() => {
      // Separate Smart Suggestions
      const suggestions: SearchableSelectItem[] = [];
      let remaining = items;

      if (smartSuggestions.length > 0) {
        suggestions.push(
          ...items.filter((item) => smartSuggestions.includes(item.id))
        );
        remaining = items.filter((item) => !smartSuggestions.includes(item.id));
      }

      // Group remaining items
      const groups: Record<string, SearchableSelectItem[]> = {};
      const ungrouped: SearchableSelectItem[] = [];

      remaining.forEach((item) => {
        if (item.group) {
          if (!groups[item.group]) {
            groups[item.group] = [];
          }
          groups[item.group].push(item);
        } else {
          ungrouped.push(item);
        }
      });

      // Sort groups alphabetically
      const sortedGroups = Object.keys(groups).sort();

      return {
        suggestions,
        groups,
        sortedGroups,
        ungrouped,
        hasSuggestions: suggestions.length > 0,
      };
    }, [items, smartSuggestions]);

    // Handle open state changes
    useEffect(() => {
      if (!open && onClose) {
        onClose();
      }
    }, [open, onClose]);

    // Set initial open state
    useEffect(() => {
      if (defaultOpen) {
        setOpen(true);
        justOpenedRef.current = true;
        // Reset the flag after a short delay to allow clicks to register
        setTimeout(() => {
          justOpenedRef.current = false;
        }, 100);
      }
    }, [defaultOpen]);

    const handleSelect = (selectedId: string) => {
      onChange(selectedId === value ? '' : selectedId);
      setOpen(false);
    };

    const popoverContentRef = useRef<HTMLDivElement>(null);
    
    return (
      <div ref={ref} className={cn('relative', className)}>
        <Popover open={open} modal={false} onOpenChange={(newOpen) => {
          setOpen(newOpen);
        }}>
          <PopoverTrigger asChild>
            <button
              type="button"
              disabled={disabled}
              className={cn(
                'flex h-8 w-full items-center rounded-md border border-input bg-background px-0.5 py-0.5 text-sm shadow-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
                open && 'ring-1 ring-ring'
              )}
            >
              <span className="flex-1 flex items-center justify-center gap-2 truncate text-center">
                {selectedItem ? (
                  <>
                    {selectedItem.icon && <span className="flex-shrink-0">{selectedItem.icon}</span>}
                    <span className="truncate text-center">{selectedItem.label}</span>
                  </>
                ) : (
                  <span className="text-muted-foreground truncate text-center">{placeholder}</span>
                )}
              </span>
              <ChevronDown className="h-4 w-4 opacity-50 flex-shrink-0 ml-auto" />
            </button>
          </PopoverTrigger>
          <PopoverContent 
            ref={popoverContentRef}
            className="w-[250px] p-0 !bg-background z-[9999] pointer-events-auto" 
            align="start"
            side="bottom"
            sideOffset={4}
            onInteractOutside={(e) => {
              const target = e.target as HTMLElement;
              
              // Always prevent closing when clicking inside the PopoverContent or on cmdk elements
              if (target.closest('[cmdk-list]') || target.closest('[cmdk-item]') || target.closest('[cmdk-group]') || target.closest('[data-radix-popover-content]') || target.closest('[data-radix-popper-content-wrapper]') || popoverContentRef.current?.contains(target)) {
                e.preventDefault();
                return;
              }
              
              // Prevent closing immediately after opening (click event might still be propagating)
              if (justOpenedRef.current) {
                e.preventDefault();
                return;
              }
              
              // Prevent closing when clicking on the table cell or any table-related elements
              if (target.closest('td') || target.closest('table') || target.closest('tbody') || target.closest('tr')) {
                e.preventDefault();
              }
            }}
            onEscapeKeyDown={(e) => {
              // Allow escape to close
            }}
            style={{ backgroundColor: 'var(--background)', opacity: 1, pointerEvents: 'auto' }}
          >
            <Command className="rounded-md border-0 !bg-background" style={{ backgroundColor: 'var(--background)', opacity: 1 }}>
              <CommandInput 
                placeholder={searchPlaceholder} 
                className="h-8 text-xs !bg-background"
                style={{ backgroundColor: 'var(--background)', opacity: 1 }}
              />
              <CommandList>
                <CommandEmpty>No options found.</CommandEmpty>
                
                {processedItems.hasSuggestions && (
                  <>
                    <CommandGroup heading="Suggested">
                      {processedItems.suggestions.map((item) => (
                        <CommandItem
                          key={`sugg-${item.id}`}
                          value={`${item.label} ${item.subLabel || ''} ${item.id}`}
                          onSelect={() => {
                            handleSelect(item.id);
                          }}
                          onClick={(e) => {
                            e.stopPropagation();
                          }}
                          className={cn(
                            'cursor-pointer',
                            value === item.id && 'bg-accent/20 font-medium'
                          )}
                        >
                          <div className="flex items-center gap-2 overflow-hidden flex-1">
                            {item.icon && <span className="flex-shrink-0">{item.icon}</span>}
                            <div className="flex flex-col truncate">
                              <span className="truncate">{item.label}</span>
                              {item.subLabel && (
                                <span className="text-xs text-muted-foreground truncate">{item.subLabel}</span>
                              )}
                            </div>
                          </div>
                          {value === item.id && <Check className="h-4 w-4 opacity-50 ml-2 flex-shrink-0" />}
                        </CommandItem>
                      ))}
                    </CommandGroup>
                    {processedItems.sortedGroups.length > 0 || processedItems.ungrouped.length > 0 ? (
                      <CommandSeparator />
                    ) : null}
                  </>
                )}

                {processedItems.sortedGroups.map((group) => (
                  <CommandGroup key={group} heading={group}>
                    {processedItems.groups[group].map((item) => (
                      <CommandItem
                        key={item.id}
                        value={`${item.label} ${item.subLabel || ''} ${item.id}`}
                        onSelect={() => {
                          handleSelect(item.id);
                        }}
                        onClick={(e) => {
                          e.stopPropagation();
                        }}
                        className={cn(
                          'cursor-pointer',
                          value === item.id && 'bg-accent/20 font-medium'
                        )}
                      >
                        <div className="flex items-center gap-2 overflow-hidden flex-1">
                          {item.icon && <span className="flex-shrink-0">{item.icon}</span>}
                          <div className="flex flex-col truncate">
                            <span className="truncate">{item.label}</span>
                            {item.subLabel && (
                              <span className="text-xs text-muted-foreground truncate">{item.subLabel}</span>
                            )}
                          </div>
                        </div>
                        {value === item.id && <Check className="h-4 w-4 opacity-50 ml-2 flex-shrink-0" />}
                      </CommandItem>
                    ))}
                  </CommandGroup>
                ))}

                {processedItems.ungrouped.length > 0 && (
                  <CommandGroup heading={processedItems.sortedGroups.length > 0 ? 'Other' : undefined}>
                    {processedItems.ungrouped.map((item) => (
                      <CommandItem
                        key={item.id}
                        value={`${item.label} ${item.subLabel || ''} ${item.id}`}
                        onSelect={() => {
                          handleSelect(item.id);
                        }}
                        onClick={(e) => {
                          e.stopPropagation();
                        }}
                        className={cn(
                          'cursor-pointer',
                          value === item.id && 'bg-accent/20 font-medium'
                        )}
                      >
                        <div className="flex items-center gap-2 overflow-hidden flex-1">
                          {item.icon && <span className="flex-shrink-0">{item.icon}</span>}
                          <div className="flex flex-col truncate">
                            <span className="truncate">{item.label}</span>
                            {item.subLabel && (
                              <span className="text-xs text-muted-foreground truncate">{item.subLabel}</span>
                            )}
                          </div>
                        </div>
                        {value === item.id && <Check className="h-4 w-4 opacity-50 ml-2 flex-shrink-0" />}
                      </CommandItem>
                    ))}
                  </CommandGroup>
                )}
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
      </div>
    );
  }
);

export default SearchableSelect;
