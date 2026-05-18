'use client';

import { useState, useRef, useEffect } from 'react';
import type { Category, ColumnMapping } from '@twocents/shared';
import { formatCurrency, normalizeVendor } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import CategorySelect from '../../components/CategorySelect';
import Input from '../../components/ui/Input';
import Button from '../../components/ui/Button';
import Select from '../../components/ui/Select';
import { Check, X, GripVertical } from 'lucide-react';

interface PreviewTransaction {
  id: string;
  date: string;
  amount: number;
  description: string;
  vendor: string | null;
  category_id: string;
}

interface ImportPreviewProps {
  transactions: PreviewTransaction[];
  categories: Category[];
  defaultPayerId: string | null;
  defaultCategoryId: string | null;
  onTransactionsChange: (transactions: PreviewTransaction[]) => void;
  onPayerChange: (payerId: string) => void;
  onImport: () => void;
  onCancel: () => void;
}

export default function ImportPreview({
  transactions,
  categories,
  defaultPayerId,
  defaultCategoryId,
  onTransactionsChange,
  onPayerChange,
  onImport,
  onCancel,
}: ImportPreviewProps) {
  const [selectedPayer, setSelectedPayer] = useState(defaultPayerId || '');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draggedCategoryId, setDraggedCategoryId] = useState<string | null>(null);
  const [draggedCategoryElement, setDraggedCategoryElement] = useState<HTMLElement | null>(null);
  const [hoveredRowId, setHoveredRowId] = useState<string | null>(null);
  const [vendorPrompt, setVendorPrompt] = useState<{
    transactionId: string;
    vendor: string;
    count: number;
    categoryId: string;
  } | null>(null);
  const tableRef = useRef<HTMLDivElement>(null);

  const updateTransaction = (id: string, updates: Partial<PreviewTransaction>) => {
    onTransactionsChange(
      transactions.map((t) => (t.id === id ? { ...t, ...updates } : t))
    );
  };

  const removeTransaction = (id: string) => {
    onTransactionsChange(transactions.filter((t) => t.id !== id));
  };

  const batchUpdateCategory = (categoryId: string) => {
    onTransactionsChange(
      transactions.map((t) => ({ ...t, category_id: categoryId }))
    );
  };

  const handleCategoryChange = (transactionId: string, categoryId: string) => {
    updateTransaction(transactionId, { category_id: categoryId });
    
    // Check for matching vendors
    const transaction = transactions.find((t) => t.id === transactionId);
    if (transaction?.vendor && categoryId) {
      const normalizedVendor = normalizeVendor(transaction.vendor);
      const matchingTransactions = transactions.filter(
        (t) => t.id !== transactionId && normalizeVendor(t.vendor || '') === normalizedVendor && !t.category_id
      );
      
      if (matchingTransactions.length > 0) {
        setVendorPrompt({
          transactionId,
          vendor: transaction.vendor,
          count: matchingTransactions.length,
          categoryId,
        });
      }
    }
  };

  const applyCategoryToVendor = (vendor: string, categoryId: string) => {
    const normalizedVendor = normalizeVendor(vendor);
    onTransactionsChange(
      transactions.map((t) =>
        normalizeVendor(t.vendor || '') === normalizedVendor && !t.category_id
          ? { ...t, category_id: categoryId }
          : t
      )
    );
    setVendorPrompt(null);
  };

  const handleCategoryDragStart = (e: React.DragEvent, categoryId: string, transactionId: string) => {
    if (!categoryId) return;
    setDraggedCategoryId(categoryId);
    setDraggedCategoryElement(e.currentTarget as HTMLElement);
    e.dataTransfer.effectAllowed = 'copy';
    e.dataTransfer.setData('text/plain', categoryId);
  };

  const handleCategoryDragEnd = () => {
    setDraggedCategoryId(null);
    setDraggedCategoryElement(null);
    setHoveredRowId(null);
  };

  const handleCategoryDrop = (e: React.DragEvent, targetTransactionId: string) => {
    e.preventDefault();
    if (draggedCategoryId) {
      updateTransaction(targetTransactionId, { category_id: draggedCategoryId });
      handleCategoryChange(targetTransactionId, draggedCategoryId);
    }
    setHoveredRowId(null);
  };

  const handleCategoryDragOver = (e: React.DragEvent, transactionId: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
    setHoveredRowId(transactionId);
  };

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Preview Transactions ({transactions.length})</CardTitle>
          <CardDescription>Review and edit transactions before importing. Drag categories to apply to multiple rows.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-4 pb-4 border-b border-border">
            <div className="flex-1">
              <label className="mb-1 block text-sm font-medium">Assign to Payer</label>
              <Select
                value={selectedPayer}
                onChange={(e) => {
                  setSelectedPayer(e.target.value);
                  onPayerChange(e.target.value);
                }}
              >
                <option value="">Select payer...</option>
                {/* Would need to fetch household members */}
              </Select>
            </div>
            <div className="flex-1">
              <label className="mb-1 block text-sm font-medium">Batch Category</label>
              <CategorySelect
                categories={categories}
                value={defaultCategoryId || ''}
                onChange={(value) => batchUpdateCategory(value)}
                placeholder="Set all categories..."
              />
            </div>
          </div>

          {/* Spreadsheet-like Table */}
          <div ref={tableRef} className="overflow-x-auto border border-border rounded-md">
            <table className="w-full border-collapse">
              <thead className="bg-muted sticky top-0 z-10">
                <tr>
                  <th className="border border-border p-2 text-left text-xs font-medium w-8"></th>
                  <th className="border border-border p-2 text-left text-xs font-medium min-w-[120px]">Date</th>
                  <th className="border border-border p-2 text-left text-xs font-medium min-w-[100px]">Amount</th>
                  <th className="border border-border p-2 text-left text-xs font-medium min-w-[200px]">Description</th>
                  <th className="border border-border p-2 text-left text-xs font-medium min-w-[150px]">Vendor</th>
                  <th className="border border-border p-2 text-left text-xs font-medium min-w-[180px]">Category</th>
                  <th className="border border-border p-2 text-left text-xs font-medium w-10"></th>
                </tr>
              </thead>
              <tbody>
                {transactions.map((transaction, index) => (
                  <tr
                    key={transaction.id}
                    className={`hover:bg-hover transition-colors ${
                      hoveredRowId === transaction.id ? 'bg-accent/10' : ''
                    }`}
                    onDragOver={(e) => handleCategoryDragOver(e, transaction.id)}
                    onDrop={(e) => handleCategoryDrop(e, transaction.id)}
                    onDragLeave={() => setHoveredRowId(null)}
                  >
                    <td className="border border-border p-1 text-center text-muted-foreground">
                      {index + 1}
                    </td>
                    <td className="border border-border p-1">
                      <Input
                        type="date"
                        value={transaction.date}
                        onChange={(e) => updateTransaction(transaction.id, { date: e.target.value })}
                        className="h-8 text-sm border-0 focus:ring-1 focus:ring-accent"
                      />
                    </td>
                    <td className="border border-border p-1">
                      <Input
                        type="number"
                        step="0.01"
                        value={transaction.amount}
                        onChange={(e) =>
                          updateTransaction(transaction.id, {
                            amount: parseFloat(e.target.value) || 0,
                          })
                        }
                        className="h-8 text-sm border-0 focus:ring-1 focus:ring-accent"
                      />
                    </td>
                    <td className="border border-border p-1">
                      <Input
                        value={transaction.description}
                        onChange={(e) =>
                          updateTransaction(transaction.id, { description: e.target.value })
                        }
                        className="h-8 text-sm border-0 focus:ring-1 focus:ring-accent"
                        placeholder="Description"
                      />
                    </td>
                    <td className="border border-border p-1 text-sm text-muted-foreground">
                      {transaction.vendor || '-'}
                    </td>
                    <td className="border border-border p-1">
                      <div className="flex items-center gap-1">
                        {transaction.category_id && (
                          <div
                            draggable
                            onDragStart={(e) => handleCategoryDragStart(e, transaction.category_id, transaction.id)}
                            onDragEnd={handleCategoryDragEnd}
                            className="cursor-grab active:cursor-grabbing flex items-center gap-1"
                            title="Drag to apply to other rows"
                          >
                            <GripVertical className="h-3 w-3 text-muted-foreground" />
                          </div>
                        )}
                        <CategorySelect
                          categories={categories}
                          value={transaction.category_id}
                          onChange={(value) => handleCategoryChange(transaction.id, value)}
                          className="h-8 text-sm flex-1 border-0 focus:ring-1 focus:ring-accent"
                        />
                      </div>
                    </td>
                    <td className="border border-border p-1 text-center">
                      <button
                        onClick={() => removeTransaction(transaction.id)}
                        className="p-1 hover:bg-destructive/10 rounded text-destructive transition-colors"
                        title="Remove transaction"
                      >
                        <X className="h-4 w-4" />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="flex justify-between items-center pt-4 border-t border-border">
            <div className="text-sm text-muted-foreground">
              Total: {formatCurrency(transactions.reduce((sum, t) => sum + t.amount, 0))}
            </div>
            <div className="flex gap-2">
              <Button variant="ghost" onClick={onCancel}>
                Cancel
              </Button>
              <Button onClick={onImport} disabled={transactions.length === 0 || !selectedPayer}>
                <Check className="mr-2 h-4 w-4" />
                Import {transactions.length} Transactions
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Vendor Prompt Modal */}
      {vendorPrompt && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 p-4">
          <Card className="w-full max-w-md">
            <CardHeader>
              <CardTitle>Apply Category to Similar Transactions?</CardTitle>
              <CardDescription>
                Found {vendorPrompt.count} other transaction{vendorPrompt.count !== 1 ? 's' : ''} with vendor "{vendorPrompt.vendor}"
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="text-sm text-muted-foreground">
                Would you like to apply this category to all {vendorPrompt.count} transaction{vendorPrompt.count !== 1 ? 's' : ''} from this vendor?
              </div>
              <div className="flex gap-2 justify-end">
                <Button
                  variant="ghost"
                  onClick={() => setVendorPrompt(null)}
                >
                  No, just this one
                </Button>
                <Button
                  onClick={() => applyCategoryToVendor(vendorPrompt.vendor, vendorPrompt.categoryId)}
                >
                  Yes, apply to all
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </>
  );
}

