'use client';

import { useState, useMemo } from 'react';
import type { Expense, Category } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Input from '../components/ui/Input';
import CategorySelect from '../components/CategorySelect';
import SearchableSelect, { SearchableSelectItem } from '../components/ui/SearchableSelect';
import Button from '../components/ui/Button';
import { X } from 'lucide-react';

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

interface ExpenseFormProps {
  expense?: Expense | null;
  categories: Category[];
  householdMembers?: HouseholdMember[];
  accounts?: Account[];
  onSubmit: (data: {
    amount: number;
    date: string;
    category_id: string;
    description: string;
    payer_id?: string;
    account_id?: string | null;
  }) => Promise<void>;
  onCancel: () => void;
}

export default function ExpenseForm({ expense, categories, householdMembers = [], accounts = [], onSubmit, onCancel }: ExpenseFormProps) {
  const [formData, setFormData] = useState({
    amount: expense?.amount ? Math.abs(expense.amount).toString() : '',
    date: expense?.date || new Date().toISOString().split('T')[0],
    category_id: expense?.category_id || '',
    description: expense?.description || '',
    payer_id: expense?.payer_id || '',
    account_id: expense?.account_id || '',
  });
  const [submitting, setSubmitting] = useState(false);

  // Convert to SearchableSelect format
  const partnerOptions = useMemo<SearchableSelectItem[]>(() => 
    householdMembers.map(m => ({
        id: m.user_id,
        label: m.profiles?.name || m.profiles?.email || 'Unknown',
        subLabel: m.profiles?.email || undefined
    })), 
  [householdMembers]);

  const accountOptions = useMemo<SearchableSelectItem[]>(() => 
    accounts.map(a => ({
        id: a.id,
        label: a.name,
        subLabel: a.type,
    })), 
  [accounts]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      await onSubmit({
        amount: Math.abs(parseFloat(formData.amount)),
        date: formData.date,
        category_id: formData.category_id,
        description: formData.description,
        payer_id: formData.payer_id || undefined,
        account_id: formData.account_id || null,
      });
      setFormData({
        amount: '',
        date: new Date().toISOString().split('T')[0],
        category_id: '',
        description: '',
        payer_id: '',
        account_id: '',
      });
    } catch (error) {
      console.error('Error saving expense:', error);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <Card className="w-full max-w-2xl">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>{expense ? 'Edit Expense' : 'New Expense'}</CardTitle>
              <CardDescription>Add or update expense details</CardDescription>
            </div>
            <button onClick={onCancel} className="rounded p-1 hover:bg-hover">
              <X className="h-5 w-5" />
            </button>
          </div>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="mb-1 block text-sm font-medium">Amount</label>
                <Input
                  type="number"
                  step="0.01"
                  required
                  value={formData.amount}
                  onChange={(e) => setFormData({ ...formData, amount: e.target.value })}
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium">Date</label>
                <Input
                  type="date"
                  required
                  value={formData.date}
                  onChange={(e) => setFormData({ ...formData, date: e.target.value })}
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium">Category</label>
                <CategorySelect
                  categories={categories}
                  value={formData.category_id}
                  onChange={(value) => setFormData({ ...formData, category_id: value })}
                  required
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium">Description</label>
                <Input
                  type="text"
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                  placeholder="Optional description"
                />
              </div>
              {householdMembers.length > 0 && (
                <div>
                  <label className="mb-1 block text-sm font-medium">Partner</label>
                  <SearchableSelect
                    items={partnerOptions}
                    value={formData.payer_id}
                    onChange={(value) => setFormData({ ...formData, payer_id: value })}
                    placeholder="Select partner..."
                  />
                </div>
              )}
              {accounts.length > 0 && (
                <div>
                  <label className="mb-1 block text-sm font-medium">Account</label>
                  <SearchableSelect
                    items={accountOptions}
                    value={formData.account_id}
                    onChange={(value) => setFormData({ ...formData, account_id: value })}
                    placeholder="Select account..."
                  />
                </div>
              )}
            </div>
            <div className="flex gap-2 justify-end">
              <Button type="button" variant="ghost" onClick={onCancel}>
                Cancel
              </Button>
              <Button type="submit" disabled={submitting}>
                {expense ? 'Update' : 'Create'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

