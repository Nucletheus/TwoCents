'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { createClient } from '@/lib/supabase/client';
import { useExpensesOptimistic, processExpenseForSavings } from '@twocents/shared';
import type { Expense, Category } from '@twocents/shared';
import ExpensesSpreadsheet from './ExpensesSpreadsheet';
import ExpenseForm from './ExpenseForm';
import ReviewSection from './ReviewSection';
import CSVImportDialog from './import/CSVImportDialog';
import MainLayout from '../components/MainLayout';
import { useHousehold } from '../components/HouseholdProvider';

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

export default function ExpensesClient() {
  const supabase = createClient();
  const { selectedHouseholdId } = useHousehold();
  const [householdMembers, setHouseholdMembers] = useState<HouseholdMember[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [toolbarPortalEl, setToolbarPortalEl] = useState<HTMLDivElement | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [showCSVImport, setShowCSVImport] = useState(false);
  const [editingExpense, setEditingExpense] = useState<Expense | null>(null);
  const [expenseFlags, setExpenseFlags] = useState<Record<string, boolean>>({});
  const [userColors, setUserColors] = useState<Record<string, string>>({});
  const [transactionsFlaggedCounter, setTransactionsFlaggedCounter] = useState(0);
  
  // Sticky grid scroll state - tracks whether the grid should be fixed to viewport
  // When true, the grid fills the remaining viewport space and scrolls internally
  const [isGridFixed, setIsGridFixed] = useState(false);
  const gridAnchorRef = useRef<HTMLDivElement>(null);
  
  // Store calculated header height and sidebar offset for fixed positioning
  const [fixedPosition, setFixedPosition] = useState({ top: 10, left: 0 });

  const { expenses, categories, loading, createExpense, updateExpense, bulkUpdateExpenses, deleteExpense, bulkDeleteExpenses, bulkImportExpenses, refetch } =
    useExpensesOptimistic(supabase, selectedHouseholdId);

  // Memoize fetch functions to prevent unnecessary re-creation and enable proper dependency tracking
  // This improves performance by avoiding function recreation on every render
  
  // Fetch household members for the selected household
  // Attempts to use Supabase relationship syntax first, falls back to separate queries if needed
  const fetchHouseholdMembers = useCallback(async () => {
    if (!selectedHouseholdId) {
      setHouseholdMembers([]);
      return;
    }

    try {
      // Query household members and join with profiles using the foreign key relationship
      // Use profiles!inner to ensure we only get members with profiles
      const { data, error } = await supabase
        .from('household_members')
        .select(`
          user_id,
          profiles!inner (
            id,
            name
          )
        `)
        .eq('household_id', selectedHouseholdId);

      if (error) {
        // If !inner fails, try without the relationship syntax and query profiles separately
        console.warn('Error with relationship query, trying alternative:', error);
        
        const { data: membersData, error: membersError } = await supabase
          .from('household_members')
          .select('user_id')
          .eq('household_id', selectedHouseholdId);

        if (membersError) {
          console.error('Error fetching household members:', membersError);
          setHouseholdMembers([]);
          return;
        }

        if (!membersData || membersData.length === 0) {
          setHouseholdMembers([]);
          return;
        }

        const userIds = membersData.map((m: any) => m.user_id);
        const { data: profilesData, error: profilesError } = await supabase
          .from('profiles')
          .select('id, name')
          .in('id', userIds);

        if (profilesError) {
          console.error('Error fetching profiles:', profilesError);
          setHouseholdMembers([]);
          return;
        }

        const { data: { user: currentUser } } = await supabase.auth.getUser();
        const profilesMap = new Map((profilesData || []).map((p: any) => [p.id, p]));

        const members = membersData.map((member: any) => {
          const profile = profilesMap.get(member.user_id);
          return {
            user_id: member.user_id,
            profiles: profile ? {
              name: profile.name,
              email: currentUser?.id === member.user_id ? (currentUser.email || '') : 'User'
            } : null,
          };
        });

        setHouseholdMembers(members);
        return;
      }

      // Ensure data structure matches expected format
      const members = (data || []).map((member: any) => {
        const profile = Array.isArray(member.profiles) ? member.profiles[0] : member.profiles;
        return {
          user_id: member.user_id,
          profiles: profile ? {
            name: profile.name,
            email: 'User' // Email not available from profiles table
          } : null,
        };
      });

      setHouseholdMembers(members);
    } catch (error) {
      console.error('Error fetching household members:', error);
      setHouseholdMembers([]);
    }
  }, [selectedHouseholdId, supabase]);

  // Fetch accounts for the selected household
  const fetchAccounts = useCallback(async () => {
    if (!selectedHouseholdId) {
      setAccounts([]);
      return;
    }
    try {
      const { data, error } = await supabase
        .from('accounts')
        .select('id, name, type, balance, user_id, household_id')
        .eq('household_id', selectedHouseholdId)
        .order('name');

      if (error) throw error;
      setAccounts(data || []);
    } catch (error) {
      console.error('Error fetching accounts:', error);
    }
  }, [selectedHouseholdId, supabase]);

  // Fetch user color preferences for the selected household
  const fetchUserColors = useCallback(async () => {
    if (!selectedHouseholdId) {
      setUserColors({});
      return;
    }
    try {
      const { data, error } = await supabase
        .from('household_member_colors')
        .select('user_id, color')
        .eq('household_id', selectedHouseholdId);

      if (error) throw error;

      const colorsMap: Record<string, string> = {};
      (data || []).forEach((item: any) => {
        colorsMap[item.user_id] = item.color;
      });
      setUserColors(colorsMap);
    } catch (error) {
      console.error('Error fetching user colors:', error);
    }
  }, [selectedHouseholdId, supabase]);

  // Fetch expense flags - depends on selectedHouseholdId and expense IDs (not the full array)
  // This prevents re-fetching when expense objects change but IDs remain the same
  const fetchExpenseFlags = useCallback(async () => {
    if (!selectedHouseholdId || expenses.length === 0) {
      setExpenseFlags({});
      return;
    }

    try {
      const expenseIds = expenses.map((e) => e.id);
      const { data, error } = await supabase
        .from('expense_flags')
        .select('expense_id')
        .in('expense_id', expenseIds)
        .is('resolved_at', null);

      if (error) throw error;

      const flagsMap: Record<string, boolean> = {};
      (data || []).forEach((flag: any) => {
        flagsMap[flag.expense_id] = true;
      });

      // Also check expense status - expenses can be flagged directly via status field
      expenses.forEach((expense) => {
        if (expense.status === 'flagged' || expense.status === 'pending_review') {
          flagsMap[expense.id] = true;
        }
      });

      setExpenseFlags(flagsMap);
    } catch (error) {
      console.error('Error fetching expense flags:', error);
    }
  }, [selectedHouseholdId, expenses, supabase]);

  // Create a new expense - handles form submission from ExpenseForm
  const handleCreate = async (data: {
    amount: number;
    date: string;
    category_id: string;
    description: string;
    payer_id?: string;
    account_id?: string | null;
  }) => {
    const {
      data: { user },
    } = await supabase.auth.getUser();

    if (!user || !selectedHouseholdId) return;

    await createExpense({
      household_id: selectedHouseholdId,
      payer_id: data.payer_id || user.id,
      account_id: data.account_id || null,
      ...data,
      amount: Math.abs(data.amount),
    });
    setShowForm(false);
  };

  // Update an existing expense - handles category changes and savings category processing
  // If the category is changed to a savings category, automatically processes it for savings tracking
  const handleUpdate = useCallback(async (id: string, data: Partial<Expense>) => {
    await updateExpense(id, data);
    
    // If category was changed, check if it's a savings category and create unallocated entry
    if (data.category_id) {
      const expense = expenses.find(e => e.id === id);
      const category = categories.find(c => c.id === data.category_id) as Category | undefined;
      
      if (expense && category && (category as any).is_savings_category) {
        // Process this expense for savings tracking
        await processExpenseForSavings(supabase, {
          id: expense.id,
          household_id: expense.household_id,
          account_id: expense.account_id,
          amount: expense.amount,
          date: expense.date,
          description: expense.description,
        }, category);
      }
    }
  }, [updateExpense, expenses, categories, supabase]);

  // Bulk delete expenses - confirmation dialog is handled in ExpensesSpreadsheet component
  const handleBulkDelete = async (ids: string[]) => {
    await bulkDeleteExpenses(ids);
  };

  // Callback to notify ReviewSection when transactions are flagged
  const handleTransactionsFlagged = useCallback(() => {
    // Increment counter to trigger refresh in ReviewGrid
    setTransactionsFlaggedCounter((prev) => prev + 1);
  }, []);

  // Fetch data when household changes - split into separate effects for better performance
  // These only depend on selectedHouseholdId, so they won't re-run when expenses change
  useEffect(() => {
    if (selectedHouseholdId) {
      fetchHouseholdMembers();
      fetchAccounts();
      fetchUserColors();
    }
  }, [selectedHouseholdId, fetchHouseholdMembers, fetchAccounts, fetchUserColors]);

  // Fetch expense flags separately - depends on expense IDs, not the full expense objects
  // This prevents unnecessary re-fetches when expense data updates but IDs stay the same
  useEffect(() => {
    if (selectedHouseholdId) {
      fetchExpenseFlags();
    }
  }, [selectedHouseholdId, fetchExpenseFlags]);

  // Event listener setup for opening forms via custom events
  // These events can be dispatched from anywhere in the app to open the expense form or CSV import
  useEffect(() => {
    const handleOpenForm = () => {
      if (selectedHouseholdId) {
        setShowForm(true);
        setEditingExpense(null);
      }
    };

    const handleOpenCSVImport = () => {
      if (selectedHouseholdId) {
        setShowCSVImport(true);
      }
    };

    window.addEventListener('open-expense-form', handleOpenForm);
    window.addEventListener('open-csv-import', handleOpenCSVImport);
    return () => {
      window.removeEventListener('open-expense-form', handleOpenForm);
      window.removeEventListener('open-csv-import', handleOpenCSVImport);
    };
  }, [selectedHouseholdId]);

  // Calculate header height and sidebar offset for fixed positioning
  // This runs when the component mounts and when window resizes
  useEffect(() => {
    const calculateFixedPosition = () => {
      // Measure header height from MainLayout's scroll container padding
      const scrollContainer = document.querySelector('[class*="overflow-y-auto"]');
      let headerHeight = 80; // Default fallback
      if (scrollContainer) {
        const style = window.getComputedStyle(scrollContainer);
        const paddingTop = parseInt(style.paddingTop, 10);
        if (paddingTop > 0) {
          headerHeight = paddingTop;
        }
      }
      
      // Get sidebar width from main element's marginLeft
      const mainElement = document.querySelector('main[class*="flex-1"]');
      let leftOffset = 0;
      if (mainElement) {
        const style = window.getComputedStyle(mainElement);
        const marginLeft = parseInt(style.marginLeft, 10);
        if (marginLeft > 0) {
          leftOffset = marginLeft;
        }
      }
      
      setFixedPosition({ top: headerHeight, left: leftOffset });
    };

    calculateFixedPosition();
    
    // Recalculate on window resize
    window.addEventListener('resize', calculateFixedPosition);
    return () => window.removeEventListener('resize', calculateFixedPosition);
  }, [selectedHouseholdId]);

  // Intersection Observer to detect when grid anchor scrolls out of view
  // When the anchor passes the sticky header, the grid becomes fixed to maximize viewport space
  useEffect(() => {
    const anchor = gridAnchorRef.current;
    if (!anchor) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        // When anchor scrolls out of view at top (above the header), fix the grid
        // entry.isIntersecting is false when anchor is above viewport
        // entry.boundingClientRect.top < 0 means anchor has scrolled past the top
        const shouldFix = !entry.isIntersecting && entry.boundingClientRect.top < 0;
        setIsGridFixed(shouldFix);
      },
      { 
        threshold: 0, 
        rootMargin: `-${fixedPosition.top}px 0px 0px 0px` // Offset for sticky header height
      }
    );

    observer.observe(anchor);
    return () => observer.disconnect();
  }, [selectedHouseholdId, fixedPosition.top]);


  // Initial load: if no ReviewSection content, immediately fix the grid
  // This provides immediate full-height grid when there are no pending reviews
  useEffect(() => {
    if (!selectedHouseholdId) return;
    
    // Check if ReviewSection has any visible content by checking if it exists in DOM
    // and has a height greater than a small threshold (e.g., 50px)
    const checkReviewSection = () => {
      const reviewSection = document.querySelector('[data-review-section]');
      if (reviewSection) {
        const height = reviewSection.getBoundingClientRect().height;
        // If ReviewSection is very small or not visible, start with fixed grid
        if (height < 50) {
          setIsGridFixed(true);
        }
      } else {
        // If ReviewSection doesn't exist, start with fixed grid
        setIsGridFixed(true);
      }
    };

    // Use a small delay to allow ReviewSection to render
    const timer = setTimeout(checkReviewSection, 100);
    return () => clearTimeout(timer);
  }, [selectedHouseholdId]);

  return (
    <MainLayout
      pageHeader={{
        title: 'Expenses',
        subtitle: 'Manage your household expenses',
        secondarySlotRef: setToolbarPortalEl,
      }}
      contentVariant="full-width"
    >
      <div className="flex flex-col gap-1">
        {showForm && (
          <ExpenseForm
            expense={editingExpense}
            categories={categories}
            householdMembers={householdMembers}
            accounts={accounts}
            onSubmit={handleCreate}
            onCancel={() => {
              setShowForm(false);
              setEditingExpense(null);
            }}
          />
        )}

        {showCSVImport && (
          <CSVImportDialog
            isOpen={showCSVImport}
            onClose={() => setShowCSVImport(false)}
            householdId={selectedHouseholdId}
            categories={categories}
            onImportComplete={async (importedExpenses) => {
              // Append imported expenses to grid without reloading
              if (importedExpenses && importedExpenses.length > 0) {
                await bulkImportExpenses(importedExpenses);
              }
              setShowCSVImport(false);
            }}
          />
        )}

        {!selectedHouseholdId ? (
          <div className="rounded-notion border border-border bg-card p-8 text-center">
            <p className="text-muted-foreground">Please select or create a household first.</p>
          </div>
        ) : (
          <>
            {/* ReviewSection scrolls naturally - users can scroll through pending reviews */}
            <div data-review-section>
              <ReviewSection
                householdId={selectedHouseholdId}
                categories={categories}
                householdMembers={householdMembers}
                accounts={accounts}
                onExpenseUpdated={() => {
                  refetch();
                }}
                transactionsFlaggedCounter={transactionsFlaggedCounter}
              />
            </div>
            
            {/* Anchor point to detect when grid should become fixed */}
            {/* When this anchor scrolls past the header, the grid switches to fixed positioning */}
            <div ref={gridAnchorRef} className="h-0" />
            
            {/* Grid with conditional fixed positioning */}
            {/* When fixed, grid fills remaining viewport space and scrolls internally */}
            <div 
              className={isGridFixed ? "fixed right-0 bottom-0 z-10" : ""}
              style={isGridFixed ? {
                top: `${fixedPosition.top}px`,
                left: `${fixedPosition.left}px`,
              } : undefined}
            >
              <ExpensesSpreadsheet
                expenses={expenses}
                categories={categories}
                householdMembers={householdMembers}
                accounts={accounts}
                toolbarPortalEl={toolbarPortalEl}
                onUpdate={handleUpdate}
                onBulkUpdate={bulkUpdateExpenses}
                onBulkDelete={handleBulkDelete}
                expenseFlags={expenseFlags}
                onFlagChange={fetchExpenseFlags}
                onTransactionsFlagged={handleTransactionsFlagged}
                householdId={selectedHouseholdId}
                userColors={userColors}
                isFixedMode={isGridFixed}
              />
            </div>
          </>
        )}
      </div>
    </MainLayout>
  );
}

