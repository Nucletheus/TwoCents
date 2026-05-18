import { useState, useEffect, useCallback, useRef } from 'react';
import type { Expense, Category } from '../types';
import { applyTransactionRules } from '../utils/transactionRules';
import { extractVendor } from '../utils/vendorExtraction';
import { normalizeCategoryGroupColors } from '../utils/colorUtils';

interface QueuedUpdate {
  id: string;
  data: Partial<Expense>;
  timestamp: number;
}

export function useExpensesOptimistic(supabase: any, householdId: string | null) {
  const [expenses, setExpenses] = useState<Expense[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const updateQueueRef = useRef<Map<string, QueuedUpdate>>(new Map());
  const debounceTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const isProcessingRef = useRef(false);

  const fetchExpenses = useCallback(async () => {
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
        .eq('household_id', householdId);
        // Sorting is now handled client-side by the user

      if (error) throw error;
      
      // Apply any pending optimistic updates
      let updatedData = data || [];
      updateQueueRef.current.forEach((queuedUpdate) => {
        const index = updatedData.findIndex((e: Expense) => e.id === queuedUpdate.id);
        if (index >= 0) {
          updatedData[index] = { ...updatedData[index], ...queuedUpdate.data };
        }
      });
      
      setExpenses(updatedData);
    } catch (err: any) {
      setError(err);
    } finally {
      setLoading(false);
    }
  }, [supabase, householdId]);

  const fetchCategories = useCallback(async () => {
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
  }, [supabase, householdId]);

  useEffect(() => {
    if (householdId) {
      fetchExpenses();
      fetchCategories();
    }
  }, [householdId, fetchExpenses, fetchCategories]);

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
  }, [householdId, fetchCategories]);

  const fetchTransactionRules = async (): Promise<any[]> => {
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

  // Process queued updates
  const processQueue = useCallback(async () => {
    if (isProcessingRef.current || updateQueueRef.current.size === 0) return;
    
    isProcessingRef.current = true;
    const queue = Array.from(updateQueueRef.current.values());
    updateQueueRef.current.clear();

    try {
      // Batch updates by expense ID
      const updatesByExpense = new Map<string, Partial<Expense>>();
      queue.forEach((update) => {
        const existing = updatesByExpense.get(update.id) || {};
        updatesByExpense.set(update.id, { ...existing, ...update.data });
      });

      // Check if any update changes the date (which would affect sorting)
      const dateChanged = Array.from(updatesByExpense.values()).some(
        (data) => 'date' in data
      );

      // Apply updates to database
      for (const [id, data] of updatesByExpense) {
        const nextData: Partial<Expense> = { ...data };
        if (typeof nextData.amount === 'number') {
          nextData.amount = Math.abs(nextData.amount);
        }

        const { error } = await supabase
          .from('expenses')
          .update(nextData)
          .eq('id', id);

        if (error) {
          console.error(`Error updating expense ${id}:`, error);
          // Re-queue failed updates
          updateQueueRef.current.set(id, { id, data, timestamp: Date.now() });
        }
      }

      // Only refetch if date changed (affects sort order), otherwise just update local state
      if (dateChanged && householdId) {
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
          .eq('household_id', householdId);
          // Sorting is now handled client-side by the user

        if (!error && data) {
          // Apply any pending optimistic updates
          let updatedData = data;
          updateQueueRef.current.forEach((queuedUpdate) => {
            const index = updatedData.findIndex((e: Expense) => e.id === queuedUpdate.id);
            if (index >= 0) {
              updatedData[index] = { ...updatedData[index], ...queuedUpdate.data };
            }
          });
          setExpenses(updatedData);
        }
      } else {
        // For non-date updates, just update local state without refetching
        setExpenses((prev) => {
          return prev.map((expense) => {
            const update = updatesByExpense.get(expense.id);
            if (update) {
              return { ...expense, ...update };
            }
            return expense;
          });
        });
      }
    } catch (err: any) {
      console.error('Error processing update queue:', err);
      // Re-queue all updates on error
      queue.forEach((update) => {
        updateQueueRef.current.set(update.id, update);
      });
    } finally {
      isProcessingRef.current = false;
    }
  }, [supabase, householdId]);

  // Debounced queue processor
  const scheduleQueueProcess = useCallback(() => {
    if (debounceTimeoutRef.current) {
      clearTimeout(debounceTimeoutRef.current);
    }
    debounceTimeoutRef.current = setTimeout(() => {
      processQueue();
    }, 500); // 500ms debounce
  }, [processQueue]);

  // Optimistic update function
  const updateExpenseOptimistic = useCallback((id: string, updates: Partial<Expense>) => {
    // Apply optimistically to local state
    setExpenses((prev) =>
      prev.map((e) => (e.id === id ? { ...e, ...updates } : e))
    );

    // Queue for database update
    const existing = updateQueueRef.current.get(id);
    updateQueueRef.current.set(id, {
      id,
      data: { ...existing?.data, ...updates },
      timestamp: Date.now(),
    });

    scheduleQueueProcess();
  }, [scheduleQueueProcess]);

  // Bulk optimistic update
  const bulkUpdateExpensesOptimistic = useCallback(
    (updates: Array<{ id: string; data: Partial<Expense> }>) => {
      // Apply optimistically to local state
      setExpenses((prev) =>
        prev.map((e) => {
          const update = updates.find((u) => u.id === e.id);
          return update ? { ...e, ...update.data } : e;
        })
      );

      // Queue for database updates
      updates.forEach(({ id, data }) => {
        const existing = updateQueueRef.current.get(id);
        updateQueueRef.current.set(id, {
          id,
          data: { ...existing?.data, ...data },
          timestamp: Date.now(),
        });
      });

      scheduleQueueProcess();
    },
    [scheduleQueueProcess]
  );

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
      
      const vendor = expenseData.vendor || (expenseData.description ? extractVendor(expenseData.description) : null);

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

  const deleteExpense = async (id: string) => {
    try {
      // Optimistically remove from local state
      setExpenses((prev) => prev.filter((e) => e.id !== id));
      
      const { error } = await supabase.from('expenses').delete().eq('id', id);

      if (error) {
        // Revert on error
        await fetchExpenses();
        throw error;
      }
    } catch (err: any) {
      setError(err);
      throw err;
    }
  };

  const bulkDeleteExpenses = async (ids: string[]) => {
    const startTime = Date.now();
    // #region agent log
    if (typeof window !== 'undefined') {
      fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'bulkDeleteExpenses entry',data:{idsCount:ids.length,ids:ids.slice(0,10),startTime},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'A,B,C,F'})}).catch(()=>{});
    }
    // #endregion
    try {
      if (ids.length === 0) return;

      // Optimistically remove all expenses from local state
      const idsSet = new Set(ids);
      const beforeOptimistic = Date.now();
      setExpenses((prev) => prev.filter((e) => !idsSet.has(e.id)));
      const afterOptimistic = Date.now();
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'optimistic update completed',data:{optimisticUpdateTime:afterOptimistic-beforeOptimistic},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'F'})}).catch(()=>{});
      }
      // #endregion

      // Delete expenses in batches to avoid query size limits and timeouts
      // PostgreSQL and Supabase have limits on query parameters, so batch deletions
      const batchSize = 100;
      const errors: any[] = [];
      let deletedCount = 0;

      // Process batches with small delays to prevent UI blocking
      // This allows the browser to process other events between batches
      for (let i = 0; i < ids.length; i += batchSize) {
        const batch = ids.slice(i, i + batchSize);
        // #region agent log
        if (typeof window !== 'undefined') {
          fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'deleting batch',data:{batchIndex:Math.floor(i/batchSize),batchSize:batch.length,batchIds:batch.slice(0,5)},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'A'})}).catch(()=>{});
        }
        // #endregion
        
        const { error, data } = await supabase.from('expenses').delete().in('id', batch).select('id');
        
        // #region agent log
        if (typeof window !== 'undefined') {
          fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'batch delete result',data:{batchIndex:Math.floor(i/batchSize),error:error?{message:error.message,code:error.code,details:error.details}:null,deletedCount:data?.length||0,batchSize:batch.length},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'B'})}).catch(()=>{});
        }
        // #endregion

        if (error) {
          errors.push({ batch, error });
          // #region agent log
          if (typeof window !== 'undefined') {
            fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'batch delete error',data:{batchIndex:Math.floor(i/batchSize),errorMessage:error.message,errorCode:error.code,errorDetails:error.details},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'C'})}).catch(()=>{});
          }
          // #endregion
        } else {
          // Only count actual deletions (data?.length), not batch size
          // RLS policies may silently filter out expenses the user can't delete
          const actualDeleted = data?.length || 0;
          deletedCount += actualDeleted;
          
          // If fewer expenses were deleted than requested, this indicates RLS filtering
          if (actualDeleted < batch.length) {
            // #region agent log
            if (typeof window !== 'undefined') {
              fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'partial batch deletion detected',data:{batchIndex:Math.floor(i/batchSize),requested:batch.length,actualDeleted,missing:batch.length-actualDeleted},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'E'})}).catch(()=>{});
            }
            // #endregion
          }
        }

        // Yield to event loop between batches to prevent UI blocking
        // Only add delay if there are more batches to process
        if (i + batchSize < ids.length) {
          await new Promise(resolve => setTimeout(resolve, 0));
        }
      }

      const deletionEndTime = Date.now();
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'bulkDeleteExpenses completed',data:{totalIds:ids.length,deletedCount,errorsCount:errors.length,hasErrors:errors.length>0,partialDeletion:deletedCount<ids.length,totalTime:deletionEndTime-startTime},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'A,B,C,E,F'})}).catch(()=>{});
      }
      // #endregion

      // Check for partial deletions (RLS may have filtered some expenses)
      if (deletedCount < ids.length) {
        const missingCount = ids.length - deletedCount;
        // Revert optimistic update since not all expenses were deleted
        await fetchExpenses();
        const errorMessage = `Only ${deletedCount} of ${ids.length} expenses were deleted. ${missingCount} expense(s) could not be deleted (you may not have permission to delete expenses you didn't pay for).`;
        const partialError = new Error(errorMessage);
        (partialError as any).deletedCount = deletedCount;
        (partialError as any).requestedCount = ids.length;
        throw partialError;
      }

      if (errors.length > 0) {
        // If some batches failed, revert optimistic update and throw
        await fetchExpenses();
        const errorMessage = errors.length === 1 
          ? errors[0].error.message 
          : `Failed to delete ${errors.length} batch(es) of expenses`;
        const combinedError = new Error(errorMessage);
        (combinedError as any).errors = errors;
        throw combinedError;
      }

      // After successful deletion, refetch to ensure UI is in sync with database
      // This is important for cases where the component might re-render or refresh
      // The optimistic update handles immediate UI feedback, but refetch ensures consistency
      // Use non-blocking refetch to avoid UI hangs - the optimistic update already removed them from UI
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'starting refetch after deletion',data:{timestamp:Date.now()},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'F'})}).catch(()=>{});
      }
      // #endregion
      // Refetch in background - don't block UI, optimistic update already handled it
      const refetchStartTime = Date.now();
      fetchExpenses().then(() => {
        const refetchEndTime = Date.now();
        // #region agent log
        if (typeof window !== 'undefined') {
          fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'refetch completed after deletion',data:{refetchTime:refetchEndTime-refetchStartTime},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'F'})}).catch(()=>{});
        }
        // #endregion
      }).catch((err) => {
        // #region agent log
        if (typeof window !== 'undefined') {
          fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'refetch error after deletion',data:{errorMessage:err?.message,refetchTime:Date.now()-refetchStartTime},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'F'})}).catch(()=>{});
        }
        // #endregion
        console.error('Error refetching expenses after deletion:', err);
      });
    } catch (err: any) {
      // #region agent log
      if (typeof window !== 'undefined') {
        fetch('http://127.0.0.1:7242/ingest/8259342f-ca9f-44b2-b8dd-43a2fbc73870',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'useExpensesOptimistic.ts:bulkDeleteExpenses',message:'bulkDeleteExpenses error caught',data:{errorMessage:err?.message,errorStack:err?.stack,errorName:err?.name},timestamp:Date.now(),sessionId:'debug-session',runId:'run1',hypothesisId:'C'})}).catch(()=>{});
      }
      // #endregion
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
      
      const batchSize = 100;
      for (let i = 0; i < expensesData.length; i += batchSize) {
        const batch = expensesData.slice(i, i + batchSize);
        
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

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (debounceTimeoutRef.current) {
        clearTimeout(debounceTimeoutRef.current);
      }
      // Process any remaining updates
      if (updateQueueRef.current.size > 0) {
        processQueue();
      }
    };
  }, [processQueue]);

  return {
    expenses,
    categories,
    loading,
    error,
    createExpense,
    updateExpense: updateExpenseOptimistic,
    bulkUpdateExpenses: bulkUpdateExpensesOptimistic,
    deleteExpense,
    bulkDeleteExpenses,
    bulkImportExpenses,
    refetch: fetchExpenses,
  };
}

