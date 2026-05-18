'use client';

import { useState } from 'react';
import type { Category } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/Card';
import Input from './ui/Input';
import Select from './ui/Select';
import CategorySelect from './CategorySelect';
import Button from './ui/Button';
import { X, Filter } from 'lucide-react';

interface FilterPanelProps {
  categories: Category[];
  vendors: string[];
  isOpen: boolean;
  onClose: () => void;
  filters: {
    category?: string;
    search?: string;
    minAmount?: number;
    maxAmount?: number;
    status?: 'pending_review' | 'approved' | 'flagged';
    dateFrom?: string;
    dateTo?: string;
    vendors?: string[];
  };
  onFiltersChange: (filters: FilterPanelProps['filters']) => void;
}

export default function FilterPanel({
  categories,
  vendors,
  isOpen,
  onClose,
  filters,
  onFiltersChange,
}: FilterPanelProps) {
  const [localFilters, setLocalFilters] = useState(filters);

  const handleApply = () => {
    onFiltersChange(localFilters);
  };

  const handleReset = () => {
    const emptyFilters = {};
    setLocalFilters(emptyFilters);
    onFiltersChange(emptyFilters);
  };

  const updateFilter = (key: keyof typeof localFilters, value: any) => {
    setLocalFilters((prev) => ({ ...prev, [key]: value }));
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <Card className="w-full max-w-2xl max-h-[90vh] overflow-y-auto">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Filter className="h-5 w-5" />
                Filters
              </CardTitle>
              <CardDescription>Filter expenses by various criteria</CardDescription>
            </div>
            <button onClick={onClose} className="rounded p-1 hover:bg-hover">
              <X className="h-5 w-5" />
            </button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <label className="mb-1 block text-sm font-medium">Search</label>
            <Input
              value={localFilters.search || ''}
              onChange={(e) => updateFilter('search', e.target.value || undefined)}
              placeholder="Search descriptions, vendors, categories..."
            />
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-1 block text-sm font-medium">Category</label>
              <CategorySelect
                categories={categories}
                value={localFilters.category || ''}
                onChange={(value) => updateFilter('category', value || undefined)}
                placeholder="All categories"
              />
            </div>

            <div>
              <label className="mb-1 block text-sm font-medium">Status</label>
              <Select
                value={localFilters.status || ''}
                onChange={(e) => updateFilter('status', e.target.value || undefined)}
              >
                <option value="">All Status</option>
                <option value="pending_review">Pending Review</option>
                <option value="approved">Approved</option>
                <option value="flagged">Flagged</option>
              </Select>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-1 block text-sm font-medium">Min Amount</label>
              <Input
                type="number"
                step="0.01"
                value={localFilters.minAmount || ''}
                onChange={(e) => updateFilter('minAmount', e.target.value ? parseFloat(e.target.value) : undefined)}
                placeholder="0.00"
              />
            </div>

            <div>
              <label className="mb-1 block text-sm font-medium">Max Amount</label>
              <Input
                type="number"
                step="0.01"
                value={localFilters.maxAmount || ''}
                onChange={(e) => updateFilter('maxAmount', e.target.value ? parseFloat(e.target.value) : undefined)}
                placeholder="0.00"
              />
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-1 block text-sm font-medium">Date From</label>
              <Input
                type="date"
                value={localFilters.dateFrom || ''}
                onChange={(e) => updateFilter('dateFrom', e.target.value || undefined)}
              />
            </div>

            <div>
              <label className="mb-1 block text-sm font-medium">Date To</label>
              <Input
                type="date"
                value={localFilters.dateTo || ''}
                onChange={(e) => updateFilter('dateTo', e.target.value || undefined)}
              />
            </div>
          </div>

          {vendors.length > 0 && (
            <div>
              <label className="mb-1 block text-sm font-medium">Vendors</label>
              <div className="max-h-32 overflow-y-auto border border-border rounded-md p-2 space-y-1">
                {vendors.map((vendor) => (
                  <label key={vendor} className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={localFilters.vendors?.includes(vendor) || false}
                      onChange={(e) => {
                        const currentVendors = localFilters.vendors || [];
                        if (e.target.checked) {
                          updateFilter('vendors', [...currentVendors, vendor]);
                        } else {
                          updateFilter('vendors', currentVendors.filter((v) => v !== vendor));
                        }
                      }}
                      className="rounded border-border"
                    />
                    <span className="text-sm">{vendor}</span>
                  </label>
                ))}
              </div>
            </div>
          )}

          <div className="flex gap-2 justify-end pt-4 border-t border-border">
            <Button type="button" variant="ghost" onClick={handleReset}>
              Reset
            </Button>
            <Button onClick={handleApply}>Apply Filters</Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

