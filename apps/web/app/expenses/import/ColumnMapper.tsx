'use client';

import { useState } from 'react';
import type { ColumnMapping } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import Select from '../../components/ui/Select';
import Button from '../../components/ui/Button';
import { ArrowRight } from 'lucide-react';

interface ColumnMapperProps {
  headers: string[];
  mapping: ColumnMapping;
  skipColumns: string[];
  onMappingChange: (mapping: ColumnMapping) => void;
  onSkipColumnsChange: (columns: string[]) => void;
  onNext: () => void;
}

export default function ColumnMapper({
  headers,
  mapping,
  skipColumns,
  onMappingChange,
  onSkipColumnsChange,
  onNext,
}: ColumnMapperProps) {
  const amountValue = Array.isArray(mapping.amount) ? (mapping.amount[0] ?? '') : (mapping.amount || '');
  const hasAmount = Array.isArray(mapping.amount) ? mapping.amount.length > 0 : Boolean(mapping.amount);

  const updateMapping = (field: keyof ColumnMapping, column: string) => {
    onMappingChange({ ...mapping, [field]: column || undefined });
  };

  const toggleSkipColumn = (column: string) => {
    if (skipColumns.includes(column)) {
      onSkipColumnsChange(skipColumns.filter((c) => c !== column));
    } else {
      onSkipColumnsChange([...skipColumns, column]);
    }
  };

  const availableColumns = headers.filter((h) => !skipColumns.includes(h));

  return (
    <Card>
      <CardHeader>
        <CardTitle>Map Columns</CardTitle>
        <CardDescription>Match CSV columns to expense fields</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-3">
          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-1 block text-sm font-medium">Date</label>
              <Select
                value={mapping.date || ''}
                onChange={(e) => updateMapping('date', e.target.value)}
              >
                <option value="">Select column...</option>
                {availableColumns.map((header) => (
                  <option key={header} value={header}>
                    {header}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <label className="mb-1 block text-sm font-medium">Amount</label>
              <Select
                value={amountValue}
                onChange={(e) => updateMapping('amount', e.target.value)}
              >
                <option value="">Select column...</option>
                {availableColumns.map((header) => (
                  <option key={header} value={header}>
                    {header}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <label className="mb-1 block text-sm font-medium">Description</label>
              <Select
                value={mapping.description || ''}
                onChange={(e) => updateMapping('description', e.target.value)}
              >
                <option value="">Select column...</option>
                {availableColumns.map((header) => (
                  <option key={header} value={header}>
                    {header}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <label className="mb-1 block text-sm font-medium">Vendor (optional)</label>
              <Select
                value={mapping.vendor || ''}
                onChange={(e) => updateMapping('vendor', e.target.value)}
              >
                <option value="">Select column...</option>
                {availableColumns.map((header) => (
                  <option key={header} value={header}>
                    {header}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <label className="mb-1 block text-sm font-medium">Category (optional)</label>
              <Select
                value={mapping.category || ''}
                onChange={(e) => updateMapping('category', e.target.value)}
              >
                <option value="">Select column...</option>
                {availableColumns.map((header) => (
                  <option key={header} value={header}>
                    {header}
                  </option>
                ))}
              </Select>
            </div>
          </div>

          <div>
            <label className="mb-2 block text-sm font-medium">Skip Columns</label>
            <div className="max-h-32 overflow-y-auto border border-border rounded-md p-2 space-y-1">
              {headers.map((header) => (
                <label key={header} className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={skipColumns.includes(header)}
                    onChange={() => toggleSkipColumn(header)}
                    className="rounded border-border"
                  />
                  <span className="text-sm">{header}</span>
                </label>
              ))}
            </div>
          </div>
        </div>

        <div className="flex justify-end pt-4 border-t border-border">
          <Button onClick={onNext} disabled={!mapping.date || !hasAmount}>
            Next: Preview
            <ArrowRight className="ml-2 h-4 w-4" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

