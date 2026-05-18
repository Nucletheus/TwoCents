'use client';

import { useState, useEffect } from 'react';
import type { TransactionRule, RuleMatchPattern, Category } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import Input from '../../components/ui/Input';
import Select from '../../components/ui/Select';
import CategorySelect from '../../components/CategorySelect';
import Button from '../../components/ui/Button';
import { X } from 'lucide-react';

interface RuleFormProps {
  rule?: TransactionRule | null;
  categories: Category[];
  onSubmit: (rule: Omit<TransactionRule, 'id' | 'created_at'>) => Promise<void>;
  onCancel: () => void;
}

export default function RuleForm({ rule, categories, onSubmit, onCancel }: RuleFormProps) {
  const [formData, setFormData] = useState({
    name: rule?.name || '',
    priority: rule?.priority || 0,
    match_type: rule?.match_type || 'vendor',
    category_id: rule?.category_id || '',
    is_active: rule?.is_active ?? true,
    // Pattern fields
    vendor: '',
    description: '',
    amount_min: '',
    amount_max: '',
    date_pattern: '',
    combination: 'AND' as 'AND' | 'OR',
  });

  useEffect(() => {
    if (rule?.match_pattern) {
      const pattern = rule.match_pattern;
      setFormData((prev) => ({
        ...prev,
        vendor: typeof pattern.vendor === 'string' ? pattern.vendor : '',
        description: typeof pattern.description === 'string' ? pattern.description : '',
        amount_min: pattern.amountRange?.min?.toString() || '',
        amount_max: pattern.amountRange?.max?.toString() || '',
        date_pattern: pattern.datePattern || '',
        combination: pattern.combination || 'AND',
      }));
    }
  }, [rule]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    const matchPattern: RuleMatchPattern = {};
    
    switch (formData.match_type) {
      case 'vendor':
        matchPattern.vendor = formData.vendor || undefined;
        break;
      case 'description':
        matchPattern.description = formData.description || undefined;
        break;
      case 'amount_range':
        matchPattern.amountRange = {
          min: formData.amount_min ? parseFloat(formData.amount_min) : undefined,
          max: formData.amount_max ? parseFloat(formData.amount_max) : undefined,
        };
        break;
      case 'date_pattern':
        matchPattern.datePattern = formData.date_pattern || undefined;
        break;
      case 'combination':
        matchPattern.combination = formData.combination;
        if (formData.vendor) matchPattern.vendor = formData.vendor;
        if (formData.description) matchPattern.description = formData.description;
        if (formData.amount_min || formData.amount_max) {
          matchPattern.amountRange = {
            min: formData.amount_min ? parseFloat(formData.amount_min) : undefined,
            max: formData.amount_max ? parseFloat(formData.amount_max) : undefined,
          };
        }
        if (formData.date_pattern) matchPattern.datePattern = formData.date_pattern;
        break;
    }

    await onSubmit({
      name: formData.name,
      priority: formData.priority,
      match_type: formData.match_type,
      match_pattern: matchPattern,
      category_id: formData.category_id || null,
      is_active: formData.is_active,
      household_id: rule?.household_id || '',
      user_id: rule?.user_id || '',
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <Card className="w-full max-w-2xl max-h-[90vh] overflow-y-auto">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>{rule ? 'Edit Rule' : 'Create Rule'}</CardTitle>
              <CardDescription>Set up automatic categorization rules</CardDescription>
            </div>
            <button onClick={onCancel} className="rounded p-1 hover:bg-hover">
              <X className="h-5 w-5" />
            </button>
          </div>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="mb-1 block text-sm font-medium">Rule Name</label>
              <Input
                required
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="e.g., Starbucks purchases"
              />
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="mb-1 block text-sm font-medium">Priority</label>
                <Input
                  type="number"
                  value={formData.priority}
                  onChange={(e) => setFormData({ ...formData, priority: parseInt(e.target.value) || 0 })}
                  placeholder="0"
                />
                <p className="mt-1 text-xs text-muted-foreground">Higher priority rules are applied first</p>
              </div>

              <div>
                <label className="mb-1 block text-sm font-medium">Match Type</label>
                <Select
                  value={formData.match_type}
                  onChange={(e) => setFormData({ ...formData, match_type: e.target.value as any })}
                >
                  <option value="vendor">Vendor Name</option>
                  <option value="description">Description</option>
                  <option value="amount_range">Amount Range</option>
                  <option value="date_pattern">Date Pattern</option>
                  <option value="combination">Combination</option>
                </Select>
              </div>
            </div>

            <div>
              <label className="mb-1 block text-sm font-medium">Category</label>
              <CategorySelect
                categories={categories}
                value={formData.category_id}
                onChange={(value) => setFormData({ ...formData, category_id: value })}
                placeholder="Select category"
              />
            </div>

            {/* Pattern fields based on match type */}
            {formData.match_type === 'vendor' && (
              <div>
                <label className="mb-1 block text-sm font-medium">Vendor Name</label>
                <Input
                  value={formData.vendor}
                  onChange={(e) => setFormData({ ...formData, vendor: e.target.value })}
                  placeholder="e.g., STARBUCKS or STARBUCKS|AMAZON"
                />
                <p className="mt-1 text-xs text-muted-foreground">Case-insensitive partial match</p>
              </div>
            )}

            {formData.match_type === 'description' && (
              <div>
                <label className="mb-1 block text-sm font-medium">Description Pattern</label>
                <Input
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                  placeholder="e.g., coffee or restaurant"
                />
                <p className="mt-1 text-xs text-muted-foreground">Case-insensitive partial match</p>
              </div>
            )}

            {formData.match_type === 'amount_range' && (
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <label className="mb-1 block text-sm font-medium">Min Amount</label>
                  <Input
                    type="number"
                    step="0.01"
                    value={formData.amount_min}
                    onChange={(e) => setFormData({ ...formData, amount_min: e.target.value })}
                    placeholder="0.00"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm font-medium">Max Amount</label>
                  <Input
                    type="number"
                    step="0.01"
                    value={formData.amount_max}
                    onChange={(e) => setFormData({ ...formData, amount_max: e.target.value })}
                    placeholder="0.00"
                  />
                </div>
              </div>
            )}

            {formData.match_type === 'date_pattern' && (
              <div>
                <label className="mb-1 block text-sm font-medium">Date Pattern</label>
                <Select
                  value={formData.date_pattern}
                  onChange={(e) => setFormData({ ...formData, date_pattern: e.target.value })}
                >
                  <option value="">Select pattern</option>
                  <option value="weekend">Weekend</option>
                  <option value="weekday">Weekday</option>
                  <option value="first-of-month">First of Month</option>
                  <option value="last-of-month">Last of Month</option>
                  <option value="monthly">Monthly (same day)</option>
                </Select>
              </div>
            )}

            {formData.match_type === 'combination' && (
              <div className="space-y-4">
                <div>
                  <label className="mb-1 block text-sm font-medium">Combination Logic</label>
                  <Select
                    value={formData.combination}
                    onChange={(e) => setFormData({ ...formData, combination: e.target.value as 'AND' | 'OR' })}
                  >
                    <option value="AND">All conditions must match (AND)</option>
                    <option value="OR">Any condition can match (OR)</option>
                  </Select>
                </div>

                <div>
                  <label className="mb-1 block text-sm font-medium">Vendor Name (optional)</label>
                  <Input
                    value={formData.vendor}
                    onChange={(e) => setFormData({ ...formData, vendor: e.target.value })}
                    placeholder="e.g., STARBUCKS"
                  />
                </div>

                <div>
                  <label className="mb-1 block text-sm font-medium">Description Pattern (optional)</label>
                  <Input
                    value={formData.description}
                    onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                    placeholder="e.g., coffee"
                  />
                </div>

                <div className="grid gap-4 md:grid-cols-2">
                  <div>
                    <label className="mb-1 block text-sm font-medium">Min Amount (optional)</label>
                    <Input
                      type="number"
                      step="0.01"
                      value={formData.amount_min}
                      onChange={(e) => setFormData({ ...formData, amount_min: e.target.value })}
                      placeholder="0.00"
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-sm font-medium">Max Amount (optional)</label>
                    <Input
                      type="number"
                      step="0.01"
                      value={formData.amount_max}
                      onChange={(e) => setFormData({ ...formData, amount_max: e.target.value })}
                      placeholder="0.00"
                    />
                  </div>
                </div>

                <div>
                  <label className="mb-1 block text-sm font-medium">Date Pattern (optional)</label>
                  <Select
                    value={formData.date_pattern}
                    onChange={(e) => setFormData({ ...formData, date_pattern: e.target.value })}
                  >
                    <option value="">None</option>
                    <option value="weekend">Weekend</option>
                    <option value="weekday">Weekday</option>
                    <option value="first-of-month">First of Month</option>
                    <option value="last-of-month">Last of Month</option>
                    <option value="monthly">Monthly</option>
                  </Select>
                </div>
              </div>
            )}

            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="is_active"
                checked={formData.is_active}
                onChange={(e) => setFormData({ ...formData, is_active: e.target.checked })}
                className="h-4 w-4 rounded border-border"
              />
              <label htmlFor="is_active" className="text-sm">
                Rule is active
              </label>
            </div>

            <div className="flex gap-2 justify-end pt-4">
              <Button type="button" variant="ghost" onClick={onCancel}>
                Cancel
              </Button>
              <Button type="submit">Save Rule</Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

