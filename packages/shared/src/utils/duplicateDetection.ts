import type { Expense } from '../types';
import { normalizeVendor, extractVendor } from './vendorExtraction';

/**
 * Check for duplicate transactions in the database
 * Matches on: same date, same amount, same normalized vendor
 */
export async function checkForDuplicates(
  supabase: any,
  householdId: string,
  transactions: Array<{
    id: string;
    date: string;
    amount: number;
    vendor: string | null;
    description?: string | null;
  }>
): Promise<Map<string, Expense[]>> {
  const duplicates = new Map<string, Expense[]>();

  if (!householdId || transactions.length === 0) {
    return duplicates;
  }

  // Process transactions in batches to avoid too many queries
  const batchSize = 50;
  for (let i = 0; i < transactions.length; i += batchSize) {
    const batch = transactions.slice(i, i + batchSize);

    // Build query for this batch
    // We'll query all expenses for this household with matching dates and amounts
    // Then filter by vendor in memory for better performance
    const dates = [...new Set(batch.map((t) => t.date))];
    const amounts = [...new Set(batch.map((t) => t.amount))];

    const { data: candidateExpenses, error } = await supabase
      .from('expenses')
      .select('*')
      .eq('household_id', householdId)
      .in('date', dates)
      .in('amount', amounts);

    if (error) {
      console.error('Error checking for duplicates:', error);
      continue;
    }

    if (!candidateExpenses || candidateExpenses.length === 0) {
      continue;
    }

    // Match transactions to candidates
    for (const transaction of batch) {
      const normalizedVendor = normalizeVendor(transaction.vendor || extractVendor(transaction.description || null) || null);
      const matchingExpenses: Expense[] = [];

      for (const expense of candidateExpenses) {
        // Exact date and amount match
        if (expense.date !== transaction.date || Number(expense.amount) !== transaction.amount) {
          continue;
        }

        // Vendor match (normalized)
        const expenseVendor = normalizeVendor(expense.vendor || extractVendor(expense.description || null) || null);
        if (normalizedVendor && expenseVendor && normalizedVendor === expenseVendor) {
          matchingExpenses.push(expense as Expense);
        } else if (!normalizedVendor && !expenseVendor) {
          // Both have no vendor, check if descriptions are similar
          const transactionDesc = (transaction.description || '').trim().toLowerCase();
          const expenseDesc = (expense.description || '').trim().toLowerCase();
          if (transactionDesc && expenseDesc && transactionDesc === expenseDesc) {
            matchingExpenses.push(expense as Expense);
          }
        }
      }

      if (matchingExpenses.length > 0) {
        duplicates.set(transaction.id, matchingExpenses);
      }
    }
  }

  return duplicates;
}

/**
 * Get the most recent category for each vendor
 * Returns a map of normalized vendor -> category_id
 */
export async function getVendorCategories(
  supabase: any,
  householdId: string,
  vendors: (string | null)[]
): Promise<Map<string, string>> {
  const vendorCategoryMap = new Map<string, string>();

  if (!householdId || vendors.length === 0) {
    return vendorCategoryMap;
  }

  // Filter out null/empty vendors
  const validVendors = vendors.filter((v): v is string => Boolean(v && v.trim()));
  if (validVendors.length === 0) {
    return vendorCategoryMap;
  }

  try {
    // Query expenses with these vendors, ordered by date DESC
    // We'll group by vendor in memory to get the most recent category for each
    const { data: expenses, error } = await supabase
      .from('expenses')
      .select('vendor, category_id, date, description')
      .eq('household_id', householdId)
      .not('category_id', 'is', null)
      .order('date', { ascending: false })
      .limit(1000); // Limit to recent expenses for performance

    if (error) {
      console.error('Error fetching vendor categories:', error);
      return vendorCategoryMap;
    }

    if (!expenses || expenses.length === 0) {
      return vendorCategoryMap;
    }

    // Create a map to track the most recent category for each normalized vendor
    const vendorMap = new Map<string, { categoryId: string; date: string }>();

    for (const expense of expenses) {
      const vendor = normalizeVendor(expense.vendor || extractVendor(expense.description || null) || null);
      if (!vendor || !expense.category_id) continue;

      // Check if this vendor is in our list of vendors to check
      const matchesVendor = validVendors.some((v) => normalizeVendor(v) === vendor);
      if (!matchesVendor) continue;

      // If we haven't seen this vendor yet, or this expense is more recent, update it
      const existing = vendorMap.get(vendor);
      if (!existing || expense.date > existing.date) {
        vendorMap.set(vendor, {
          categoryId: expense.category_id,
          date: expense.date,
        });
      }
    }

    // Convert to the return format
    for (const [vendor, { categoryId }] of vendorMap) {
      vendorCategoryMap.set(vendor, categoryId);
    }
  } catch (error) {
    console.error('Error in getVendorCategories:', error);
  }

  return vendorCategoryMap;
}

