import { useState, useEffect } from 'react';
import type { Expense, Category, TransactionRule } from '../types';
import { applyTransactionRules } from '../utils/transactionRules';
import { extractVendor } from '../utils/vendorExtraction';
import { normalizeCategoryGroupColors } from '../utils/colorUtils';

export function useExpenses(supabase: any, householdId: string | null) {
  const [expenses, setExpenses] = useState<Expense[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (householdId) {
      fetchExpenses();
      fetchCategories();
    }
  }, [householdId]);

  useEffect(() => {
    if (!householdId) return;
    const handler = (e: Event) => {
      const detail = (e as CustomEvent)?.detail as { householdId?: string } | undefined;
      if (detail?.householdId && detail.householdId === householdId) {
        fetchCategories();
      }
    };

    window.addEventListener('category-group-colors-updated', handler as EventListener);
    return () => window.removeEventListener('category-group-colors-updated', handler as EventListener);
  }, [householdId]);

  const fetchExpenses = async () => {
    if (!householdId) return;

    try {
      const { data, error } = await supabase
        .from('expenses')
        .select(
          `
          *,
          categories (
            id,
            name,
            icon,
            color,
            group_name,
            parent_color
          )
        `
        )
        .eq('household_id', householdId)
        .order('date', { ascending: false });

      if (error) throw error;
      setExpenses(data || []);
    } catch (err: any) {
      setError(err);
    } finally {
      setLoading(false);
    }
  };

  const fetchCategories = async () => {
    try {
      if (!householdId) return;

      const { data: householdCategories, error: householdError } = await supabase
        .from('categories')
        .select('*')
        .eq('household_id', householdId)
        .neq('is_hidden', true)
        .order('group_name')
        .order('name');

      if (householdError) throw householdError;

      let overrides: Record<string, string> | undefined;
      const { data: overrideRows, error: overrideError } = await supabase
        .from('household_category_group_colors')
        .select('group_name, color')
        .eq('household_id', householdId);

      if (!overrideError && overrideRows) {
        overrides = Object.fromEntries(
          overrideRows.map((row: any) => [String(row.group_name), String(row.color)])
        );
      }

      setCategories(normalizeCategoryGroupColors((householdCategories || []) as any, overrides));
    } catch (err: any) {
      console.error('Error fetching categories:', err);
    }
  };

  const fetchTransactionRules = async (): Promise<TransactionRule[]> => {
    if (!householdId) return [];

    try {
      const { data, error } = await supabase
        .from('transaction_rules')
        .select('*')
        .eq('household_id', householdId)
        .eq('is_active', true)
        .order('priority', { ascending: false });

      if (error) throw error;
      return data || [];
    } catch (err: any) {
      console.error('Error fetching transaction rules:', err);
      return [];
    }
  };

  const createExpense = async (expenseData: {
    household_id: string;
    payer_id: string;
    amount: number;
    category_id?: string;
    description?: string;
    sub_description?: string;
    date: string;
    vendor?: string;
    status?: 'pending_review' | 'approved' | 'flagged';
  }) => {
    try {
      // Amounts are stored as absolute values in the DB (constraint: amount >= 0)
      const normalizedAmount = Math.abs(expenseData.amount);
      
      // Extract vendor if not provided
      const vendor = expenseData.vendor || (expenseData.description ? extractVendor(expenseData.description) : null);

      // Apply transaction rules if no category provided
      let categoryId = expenseData.category_id;
      if (!categoryId) {
        const rules = await fetchTransactionRules();
        const tempExpense: Expense = {
          id: '',
          household_id: expenseData.household_id,
          payer_id: expenseData.payer_id,
          account_id: null,
          amount: normalizedAmount,
          category_id: '',
          description: expenseData.description || null,
          sub_description: expenseData.sub_description || null,
          vendor,
          status: expenseData.status || 'approved',
          reviewed_by: null,
          reviewed_at: null,
          date: expenseData.date,
          receipt_url: null,
          created_at: new Date().toISOString(),
        };
        categoryId = applyTransactionRules(tempExpense, rules) || undefined;
      }

      const { data, error } = await supabase
        .from('expenses')
        .insert({
          ...expenseData,
          amount: normalizedAmount,
          category_id: categoryId,
          vendor,
          status: expenseData.status || 'approved',
        })
        .select()
        .single();

      if (error) throw error;
      await fetchExpenses();
      return data;
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  const updateExpense = async (id: string, updates: Partial<Expense>) => {
    try {
      const nextUpdates: Partial<Expense> = { ...updates };
      if (typeof nextUpdates.amount === 'number') {
        nextUpdates.amount = Math.abs(nextUpdates.amount);
      }

      const { error } = await supabase
        .from('expenses')
        .update(nextUpdates)
        .eq('id', id);

      if (error) throw error;
      await fetchExpenses();
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  const deleteExpense = async (id: string) => {
    try {
      const { error } = await supabase.from('expenses').delete().eq('id', id);

      if (error) throw error;
      await fetchExpenses();
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  const createCategory = async (categoryData: {
    name: string;
    icon?: string;
    color?: string;
    household_id?: string;
  }) => {
    try {
      const targetHouseholdId = categoryData.household_id || householdId;
      if (!targetHouseholdId) {
        throw new Error('household_id is required to create a category');
      }
      const { data, error } = await supabase
        .from('categories')
        .insert({ ...categoryData, household_id: targetHouseholdId, is_default: false })
        .select()
        .single();

      if (error) throw error;
      await fetchCategories();
      return data;
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  const bulkImportExpenses = async (expensesData: Array<{
    household_id: string;
    payer_id: string;
    amount: number;
    category_id?: string;
    description?: string;
    date: string;
    vendor?: string;
    status?: 'pending_review' | 'approved' | 'flagged';
  }>) => {
    try {
      if (expensesData.length === 0) return;

      // Import in batches of 100 for performance
      const batchSize = 100;
      for (let i = 0; i < expensesData.length; i += batchSize) {
        const batch = expensesData.slice(i, i + batchSize);
        
        // Apply rules to each expense in batch
        const rules = await fetchTransactionRules();
        const processedBatch = await Promise.all(
          batch.map(async (expenseData) => {
            // Preserve negative values - use absolute only for rule matching
            const absoluteAmount = Math.abs(expenseData.amount);

            let categoryId = expenseData.category_id;
            if (!categoryId) {
              const tempExpense: Expense = {
                id: '',
                household_id: expenseData.household_id,
                payer_id: expenseData.payer_id,
                account_id: null,
                amount: absoluteAmount,
                category_id: '',
                description: expenseData.description || null,
                sub_description: null,
                vendor: expenseData.vendor || null,
                status: expenseData.status || 'approved',
                reviewed_by: null,
                reviewed_at: null,
                date: expenseData.date,
                receipt_url: null,
                created_at: new Date().toISOString(),
              };
              categoryId = applyTransactionRules(tempExpense, rules) || undefined;
            }

            return {
              ...expenseData,
              amount: expenseData.amount, // Preserve original sign (negative values)
              category_id: categoryId,
              vendor: expenseData.vendor || (expenseData.description ? extractVendor(expenseData.description) : null),
            };
          })
        );

        const { error } = await supabase.from('expenses').insert(processedBatch);
        if (error) throw error;
      }

      await fetchExpenses();
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  return {
    expenses,
    categories,
    loading,
    error,
    createExpense,
    updateExpense,
    deleteExpense,
    createCategory,
    bulkImportExpenses,
    refetch: fetchExpenses,
  };
}

