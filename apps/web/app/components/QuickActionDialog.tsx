'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import { createPortal } from 'react-dom';
import { useRouter } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
import { useExpenses, useSavingsGoals, useHouseholds } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/Card';
import Button from './ui/Button';
import Select from './ui/Select';
import Input from './ui/Input';
import QuickActionGrid from './QuickActionGrid';
import { useHousehold } from './HouseholdProvider';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from './ui/dialog';
import { X, FileUp, Target, Users } from 'lucide-react';

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

interface QuickActionDialogProps {
  isOpen: boolean;
  onClose: () => void;
  selectedHouseholdId: string | null;
}

export default function QuickActionDialog({
  isOpen,
  onClose,
  selectedHouseholdId: initialHouseholdId,
}: QuickActionDialogProps) {
  const router = useRouter();
  const supabase = createClient();
  const { households, selectedHouseholdId, setSelectedHouseholdId, refetch: refetchGlobalHouseholds } = useHousehold();
  const { categories, bulkImportExpenses } = useExpenses(supabase, selectedHouseholdId);
  const { createGoal } = useSavingsGoals(supabase, selectedHouseholdId);
  const { createHousehold } = useHouseholds(supabase);
  const [householdMembers, setHouseholdMembers] = useState<HouseholdMember[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [defaultPayerId, setDefaultPayerId] = useState<string | null>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  
  // Dialog states
  const [showGoalDialog, setShowGoalDialog] = useState(false);
  const [showHouseholdDialog, setShowHouseholdDialog] = useState(false);
  const [savingsAccounts, setSavingsAccounts] = useState<Account[]>([]);
  
  // Form states
  const [goalFormData, setGoalFormData] = useState({
    name: '',
    target_amount: '',
    deadline: '',
    account_id: '',
  });
  const [householdFormData, setHouseholdFormData] = useState({
    name: '',
  });
  const [isCreatingGoal, setIsCreatingGoal] = useState(false);
  const [isCreatingHousehold, setIsCreatingHousehold] = useState(false);

  useEffect(() => {
    if (isOpen && selectedHouseholdId) {
      fetchHouseholdMembers();
      fetchAccounts();
      fetchSavingsAccounts();
      fetchCurrentUser();
    } else if (isOpen) {
      setHouseholdMembers([]);
      setAccounts([]);
      setSavingsAccounts([]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, selectedHouseholdId]);
  
  const fetchSavingsAccounts = async () => {
    if (!selectedHouseholdId) {
      setSavingsAccounts([]);
      return;
    }
    try {
      const { data, error } = await supabase
        .from('accounts')
        .select('*')
        .eq('household_id', selectedHouseholdId)
        .eq('type', 'savings')
        .order('name');
      if (error) throw error;
      setSavingsAccounts(data || []);
    } catch (error) {
      console.error('Error fetching savings accounts:', error);
    }
  };

  const fetchCurrentUser = async () => {
    try {
      const { data: { user } } = await supabase.auth.getUser();
      if (user) {
        setDefaultPayerId(user.id);
      }
    } catch (error) {
      console.error('Error fetching current user:', error);
    }
  };

  const fetchHouseholdMembers = useCallback(async () => {
    if (!selectedHouseholdId) {
      setHouseholdMembers([]);
      return;
    }

    try {
      console.log('Fetching household members for household:', selectedHouseholdId);
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
          console.log('No household members found');
          setHouseholdMembers([]);
          return;
        }

        console.log('Found household members:', membersData.length);
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

        console.log('Found profiles:', profilesData?.length || 0);
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

        console.log('Setting household members:', members.length);
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

      console.log('Setting household members (relationship query):', members.length);
      setHouseholdMembers(members);
    } catch (error) {
      console.error('Error fetching household members:', error);
      setHouseholdMembers([]);
    }
  }, [selectedHouseholdId]);

  const fetchAccounts = async () => {
    if (!selectedHouseholdId) return;
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
  };

  const handleSave = async (rows: Array<{
    date: string;
    amount: number;
    category_id: string;
    description?: string;
    payer_id: string;
    account_id?: string | null;
  }>) => {
    if (!selectedHouseholdId) return;

    const expensesToCreate = rows.map((row) => ({
      household_id: selectedHouseholdId,
      payer_id: row.payer_id,
      account_id: row.account_id || null,
      amount: row.amount,
      date: row.date,
      category_id: row.category_id,
      description: row.description || null,
    }));

    await bulkImportExpenses(expensesToCreate);
    onClose();
  };

  const handleImportCSV = () => {
    onClose();
    const event = new CustomEvent('open-csv-import');
    window.dispatchEvent(event);
  };

  const handleAddGoal = () => {
    setShowGoalDialog(true);
  };

  const handleAddHousehold = () => {
    setShowHouseholdDialog(true);
  };

  const handleGoalSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedHouseholdId) return;

    setIsCreatingGoal(true);
    try {
      await createGoal({
        household_id: selectedHouseholdId,
        name: goalFormData.name,
        target_amount: parseFloat(goalFormData.target_amount),
        deadline: goalFormData.deadline || undefined,
        account_id: goalFormData.account_id || null,
      });
      setShowGoalDialog(false);
      setGoalFormData({ name: '', target_amount: '', deadline: '', account_id: '' });
    } catch (error) {
      console.error('Error creating goal:', error);
    } finally {
      setIsCreatingGoal(false);
    }
  };

  const handleHouseholdSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!householdFormData.name.trim()) return;

    setIsCreatingHousehold(true);
    try {
      await createHousehold(householdFormData.name.trim());
      setShowHouseholdDialog(false);
      setHouseholdFormData({ name: '' });
      refetchGlobalHouseholds();
    } catch (error) {
      console.error('Error creating household:', error);
    } finally {
      setIsCreatingHousehold(false);
    }
  };

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const isNestedDialogOpen = showGoalDialog || showHouseholdDialog;

  const dialogContent = (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget && !isNestedDialogOpen) onClose();
      }}
      onKeyDown={(e) => {
        if (e.key === 'Escape' && !isNestedDialogOpen) onClose();
      }}
    >
      <div className="relative w-full max-w-4xl">
        <Card 
          ref={dialogRef} 
          className={`w-full max-h-[90vh] overflow-hidden flex flex-col transition-opacity ${
            isNestedDialogOpen ? 'opacity-50 pointer-events-none' : ''
          }`}
        >
        <CardHeader className="flex-shrink-0">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Quick Actions</CardTitle>
              <CardDescription>Add expense or perform other quick actions</CardDescription>
            </div>
            <button
              onClick={onClose}
              className="rounded p-1 hover:bg-hover transition-colors"
              aria-label="Close"
            >
              <X className="h-5 w-5" />
            </button>
          </div>
        </CardHeader>

        {/* Household Selector */}
        {households.length > 0 && (
          <div className="px-6 pb-4 flex-shrink-0 border-b border-border">
            <div className="flex items-center gap-2">
              <label className="text-sm font-medium">Household:</label>
              <Select
                value={selectedHouseholdId || ''}
                onChange={(e) => setSelectedHouseholdId(e.target.value || null)}
                className="h-9 w-48"
              >
                <option value="">Select household...</option>
                {households.map((h) => (
                  <option key={h.id} value={h.id}>
                    {h.name}
                  </option>
                ))}
              </Select>
            </div>
          </div>
        )}

        {/* Horizontal scrolling quick actions */}
        <div className="px-6 pb-4 flex-shrink-0 border-b border-border">
          <div className="flex gap-2 overflow-x-auto scrollbar-hide pb-2">
            <Button
              onClick={handleImportCSV}
              variant="secondary"
              size="sm"
              className="flex-shrink-0 rounded-lg bg-accent/10 text-accent border border-accent/20 hover:bg-accent/20 hover:border-accent/30 transition-all pt-0 pb-0 mt-1 mb-1"
              style={{ backgroundColor: 'rgba(59, 59, 59, 1)' }}
            >
              <FileUp className="mr-2 h-4 w-4" />
              Import CSV
            </Button>
            <Button
              onClick={handleAddGoal}
              variant="secondary"
              size="sm"
              className="flex-shrink-0 rounded-lg bg-accent/10 text-accent border border-accent/20 hover:bg-accent/20 hover:border-accent/30 transition-all pt-0 pb-0 mt-1 mb-1"
              style={{ backgroundColor: 'rgba(59, 59, 59, 1)' }}
              >
              <Target className="mr-2 h-4 w-4" />
              Add Goal
            </Button>
            <Button
              onClick={handleAddHousehold}
              variant="secondary"
              size="sm"
              className="flex-shrink-0 rounded-lg bg-accent/10 text-accent border border-accent/20 hover:bg-accent/20 hover:border-accent/30 transition-all pt-0 pb-0 mt-1 mb-1"
              style={{ backgroundColor: 'rgba(59, 59, 59, 1)' }}
              >
              <Users className="mr-2 h-4 w-4" />
              Add Household
            </Button>
          </div>
        </div>

        {/* Expense Grid */}
        <CardContent className="flex-1 overflow-y-auto">
          {!selectedHouseholdId ? (
            <div className="py-8 text-center">
              <p className="text-muted-foreground mb-4">
                Please select a household to add expenses.
              </p>
              <Button
                type="button"
                variant="secondary"
                onClick={() => {
                  onClose();
                  router.push('/households');
                }}
              >
                <Users className="mr-2 h-4 w-4" />
                Go to Households
              </Button>
            </div>
          ) : (
            <QuickActionGrid
              categories={categories}
              householdMembers={householdMembers}
              accounts={accounts}
              selectedHouseholdId={selectedHouseholdId}
              defaultPayerId={defaultPayerId}
              onSave={handleSave}
            />
          )}
        </CardContent>
      </Card>
      {isNestedDialogOpen && (
        <div className="absolute inset-0 bg-background/80 backdrop-blur-sm rounded-lg pointer-events-none" />
      )}
      </div>
    </div>
  );

  // Render in portal to ensure it's always on top
  const portalContent = (
    <>
      {typeof window !== 'undefined' && createPortal(dialogContent, document.body)}
      
      {/* Savings Goal Dialog */}
      <Dialog open={showGoalDialog} onOpenChange={setShowGoalDialog}>
        <DialogContent className="z-[110]">
          <DialogHeader>
            <DialogTitle>Create Savings Goal</DialogTitle>
            <DialogDescription>
              Set a new financial target to work towards.
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleGoalSubmit} className="space-y-4">
            <div>
              <label className="mb-1.5 block text-sm font-medium">Goal Name</label>
              <Input
                type="text"
                value={goalFormData.name}
                onChange={(e) => setGoalFormData({ ...goalFormData, name: e.target.value })}
                placeholder="e.g., Emergency Fund, Vacation, New Car"
                required
              />
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="mb-1.5 block text-sm font-medium">Target Amount</label>
                <Input
                  type="number"
                  step="0.01"
                  min="0"
                  value={goalFormData.target_amount}
                  onChange={(e) => setGoalFormData({ ...goalFormData, target_amount: e.target.value })}
                  placeholder="0.00"
                  required
                />
              </div>
              <div>
                <label className="mb-1.5 block text-sm font-medium">Deadline (optional)</label>
                <Input
                  type="date"
                  value={goalFormData.deadline}
                  onChange={(e) => setGoalFormData({ ...goalFormData, deadline: e.target.value })}
                />
              </div>
            </div>

            <div>
              <label className="mb-1.5 block text-sm font-medium">
                Link to Savings Account (optional)
              </label>
              <Select
                value={goalFormData.account_id}
                onChange={(e) => setGoalFormData({ ...goalFormData, account_id: e.target.value })}
              >
                <option value="">No account linked</option>
                {savingsAccounts.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name}
                  </option>
                ))}
              </Select>
              <p className="mt-1 text-xs text-muted-foreground">
                When linked, deposits to this account will automatically fill this goal by priority.
              </p>
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="ghost"
                onClick={() => {
                  setShowGoalDialog(false);
                  setGoalFormData({ name: '', target_amount: '', deadline: '', account_id: '' });
                }}
                disabled={isCreatingGoal}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isCreatingGoal || !selectedHouseholdId}>
                {isCreatingGoal ? 'Creating...' : 'Create Goal'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Household Dialog */}
      <Dialog open={showHouseholdDialog} onOpenChange={setShowHouseholdDialog}>
        <DialogContent className="z-[110]">
          <DialogHeader>
            <DialogTitle>Create New Household</DialogTitle>
            <DialogDescription>
              Add a new household to start tracking expenses together.
            </DialogDescription>
          </DialogHeader>
          <form onSubmit={handleHouseholdSubmit}>
            <div className="py-4">
              <label className="block text-sm font-medium mb-2">
                Household Name
              </label>
              <Input
                type="text"
                value={householdFormData.name}
                onChange={(e) => setHouseholdFormData({ name: e.target.value })}
                placeholder="e.g., Smith Family, Our Home"
                disabled={isCreatingHousehold}
                autoFocus
              />
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="ghost"
                onClick={() => {
                  setShowHouseholdDialog(false);
                  setHouseholdFormData({ name: '' });
                }}
                disabled={isCreatingHousehold}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isCreatingHousehold || !householdFormData.name.trim()}>
                {isCreatingHousehold ? 'Creating...' : 'Create Household'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );

  if (typeof window !== 'undefined') {
    return portalContent;
  }

  return null;
}
