'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { createClient } from '@/lib/supabase/client';
import { useSavingsGoals, formatCurrency, formatDate } from '@twocents/shared';
import MainLayout from '../components/MainLayout';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Input from '../components/ui/Input';
import Button from '../components/ui/Button';
import Select from '../components/ui/Select';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../components/ui/dialog';
import {
  Plus,
  Target,
  X,
  GripVertical,
  Wallet,
  Calendar,
  TrendingUp,
  CheckCircle2,
  AlertCircle,
  Trash2,
  Edit3,
  Link2,
  Unlink,
  ChevronRight,
} from 'lucide-react';
import { useHousehold } from '../components/HouseholdProvider';
import type { Account, SavingsGoal, UnallocatedSavings } from '@twocents/shared';

// Circular Progress Ring Component
function ProgressRing({
  percentage,
  size = 80,
  strokeWidth = 6,
  className = '',
}: {
  percentage: number;
  size?: number;
  strokeWidth?: number;
  className?: string;
}) {
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const offset = circumference - (percentage / 100) * circumference;

  return (
    <div className={`relative ${className}`} style={{ width: size, height: size }}>
      <svg width={size} height={size} className="transform -rotate-90">
        {/* Background circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          className="text-muted/50"
        />
        {/* Progress circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="url(#progressGradient)"
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          className="transition-all duration-500 ease-out"
        />
        <defs>
          <linearGradient id="progressGradient" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="var(--accent)" />
            <stop offset="100%" stopColor="var(--palette-secondary)" />
          </linearGradient>
        </defs>
      </svg>
      <div className="absolute inset-0 flex items-center justify-center">
        <span className="text-sm font-semibold">{Math.round(percentage)}%</span>
      </div>
    </div>
  );
}

// Goal Card Component
function GoalCard({
  goal,
  progress,
  account,
  onEdit,
  onDelete,
  onAddContribution,
  isDragging,
  dragHandleProps,
}: {
  goal: SavingsGoal;
  progress: { current: number; percentage: number; remaining: number };
  account?: Account | null;
  onEdit: () => void;
  onDelete: () => void;
  onAddContribution: (amount: number) => void;
  isDragging: boolean;
  dragHandleProps: any;
}) {
  const [contributionAmount, setContributionAmount] = useState('');
  const isCompleted = progress.percentage >= 100;

  const handleAddContribution = () => {
    const amount = parseFloat(contributionAmount);
    if (amount > 0) {
      onAddContribution(amount);
      setContributionAmount('');
    }
  };

  // Calculate days until deadline
  const daysRemaining = goal.deadline
    ? Math.ceil((new Date(goal.deadline).getTime() - Date.now()) / (1000 * 60 * 60 * 24))
    : null;

  return (
    <Card
      className={`group relative transition-all duration-200 ${
        isDragging ? 'opacity-50 scale-[1.02] shadow-lg ring-2 ring-accent' : ''
      } ${isCompleted ? 'bg-gradient-to-br from-green-500/5 to-emerald-500/5' : ''}`}
    >
      {/* Drag Handle */}
      <div
        {...dragHandleProps}
        className="absolute left-0 top-0 bottom-0 w-8 flex items-center justify-center cursor-grab active:cursor-grabbing opacity-0 group-hover:opacity-100 transition-opacity"
      >
        <GripVertical className="h-4 w-4 text-muted-foreground" />
      </div>

      <CardContent className="p-4 pl-8">
        <div className="flex gap-4">
          {/* Progress Ring */}
          <div className="flex-shrink-0">
            <ProgressRing percentage={progress.percentage} size={72} strokeWidth={5} />
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2 mb-1">
              <div className="flex items-center gap-2 min-w-0">
                {isCompleted ? (
                  <CheckCircle2 className="h-4 w-4 text-green-500 flex-shrink-0" />
                ) : (
                  <Target className="h-4 w-4 text-accent flex-shrink-0" />
                )}
                <h3 className="font-semibold truncate">{goal.name}</h3>
              </div>

              {/* Actions */}
              <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={onEdit}
                  className="p-1 rounded hover:bg-hover transition-colors"
                  title="Edit goal"
                >
                  <Edit3 className="h-3.5 w-3.5 text-muted-foreground" />
                </button>
                <button
                  onClick={onDelete}
                  className="p-1 rounded hover:bg-hover transition-colors text-red-500"
                  title="Delete goal"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>

            {/* Amount Progress */}
            <div className="text-sm text-muted-foreground mb-2">
              <span className="font-medium text-foreground">{formatCurrency(progress.current)}</span>
              <span> / {formatCurrency(goal.target_amount)}</span>
              {progress.remaining > 0 && (
                <span className="ml-2 text-xs">({formatCurrency(progress.remaining)} to go)</span>
              )}
            </div>

            {/* Meta Info */}
            <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
              {account && (
                <div className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-accent/10 text-accent">
                  <Link2 className="h-3 w-3" />
                  <span>{account.name}</span>
                </div>
              )}

              {goal.deadline && (
                <div className={`flex items-center gap-1 ${daysRemaining && daysRemaining < 30 ? 'text-amber-500' : ''}`}>
                  <Calendar className="h-3 w-3" />
                  <span>
                    {daysRemaining && daysRemaining > 0
                      ? `${daysRemaining} days left`
                      : daysRemaining === 0
                      ? 'Due today'
                      : 'Overdue'}
                  </span>
                </div>
              )}

              <div className="flex items-center gap-1 text-muted-foreground/70">
                <span>Priority #{(goal.priority || 0) + 1}</span>
              </div>
            </div>

            {/* Quick Add Contribution */}
            {!isCompleted && (
              <div className="mt-3 flex gap-2">
                <Input
                  type="number"
                  step="0.01"
                  min="0"
                  placeholder="Add amount..."
                  value={contributionAmount}
                  onChange={(e) => setContributionAmount(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      handleAddContribution();
                    }
                  }}
                  className="h-8 text-sm flex-1"
                />
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={handleAddContribution}
                  disabled={!contributionAmount || parseFloat(contributionAmount) <= 0}
                  className="h-8 px-3"
                >
                  <Plus className="h-4 w-4" />
                </Button>
              </div>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

// Unallocated Savings Item
function UnallocatedItem({
  item,
  onAllocate,
}: {
  item: UnallocatedSavings;
  onAllocate: () => void;
}) {
  return (
    <div className="flex items-center justify-between p-3 rounded-lg bg-muted/30 hover:bg-muted/50 transition-colors">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <Wallet className="h-4 w-4 text-amber-500" />
          <span className="font-medium">{formatCurrency(item.amount)}</span>
        </div>
        <div className="text-xs text-muted-foreground mt-0.5">
          {formatDate(item.date)}
          {item.description && <span className="ml-2">• {item.description}</span>}
        </div>
      </div>
      <Button size="sm" variant="ghost" onClick={onAllocate} className="h-7 text-xs">
        Assign
        <ChevronRight className="h-3 w-3 ml-1" />
      </Button>
    </div>
  );
}

// Summary Stats Component
function SummaryStats({
  stats,
}: {
  stats: {
    totalSaved: number;
    totalTarget: number;
    completedGoals: number;
    activeGoals: number;
    unallocatedTotal: number;
    overallPercentage: number;
  };
}) {
  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
      <Card className="p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-accent/10">
            <TrendingUp className="h-5 w-5 text-accent" />
          </div>
          <div>
            <p className="text-xs text-muted-foreground">Total Saved</p>
            <p className="text-lg font-semibold">{formatCurrency(stats.totalSaved)}</p>
          </div>
        </div>
      </Card>

      <Card className="p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-green-500/10">
            <CheckCircle2 className="h-5 w-5 text-green-500" />
          </div>
          <div>
            <p className="text-xs text-muted-foreground">Completed</p>
            <p className="text-lg font-semibold">{stats.completedGoals} goals</p>
          </div>
        </div>
      </Card>

      <Card className="p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-blue-500/10">
            <Target className="h-5 w-5 text-blue-500" />
          </div>
          <div>
            <p className="text-xs text-muted-foreground">Active Goals</p>
            <p className="text-lg font-semibold">{stats.activeGoals}</p>
          </div>
        </div>
      </Card>

      {stats.unallocatedTotal > 0 && (
        <Card className="p-4 border-amber-500/30 bg-amber-500/5">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-amber-500/10">
              <AlertCircle className="h-5 w-5 text-amber-500" />
            </div>
            <div>
              <p className="text-xs text-muted-foreground">Unallocated</p>
              <p className="text-lg font-semibold">{formatCurrency(stats.unallocatedTotal)}</p>
            </div>
          </div>
        </Card>
      )}
    </div>
  );
}

export default function GoalsClient() {
  const supabase = createClient();
  const { selectedHouseholdId } = useHousehold();
  const [showForm, setShowForm] = useState(false);
  const [editingGoal, setEditingGoal] = useState<SavingsGoal | null>(null);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [allocateModalOpen, setAllocateModalOpen] = useState(false);
  const [selectedUnallocated, setSelectedUnallocated] = useState<UnallocatedSavings | null>(null);
  const [userId, setUserId] = useState<string | null>(null);

  // Drag state
  const [draggedGoalId, setDraggedGoalId] = useState<string | null>(null);
  const [dragOverGoalId, setDragOverGoalId] = useState<string | null>(null);

  const [formData, setFormData] = useState({
    name: '',
    target_amount: '',
    deadline: '',
    account_id: '',
  });

  const {
    goals,
    contributions,
    unallocatedSavings,
    loading,
    createGoal,
    updateGoal,
    deleteGoal,
    reorderGoals,
    addContribution,
    allocateSavings,
    getGoalProgress,
    getSummaryStats,
  } = useSavingsGoals(supabase, selectedHouseholdId);

  // Fetch user ID
  useEffect(() => {
    const fetchUser = async () => {
      const { data: { user } } = await supabase.auth.getUser();
      setUserId(user?.id || null);
    };
    fetchUser();
  }, [supabase]);

  // Fetch accounts (only savings type for linking)
  useEffect(() => {
    const fetchAccounts = async () => {
      if (!selectedHouseholdId) {
        setAccounts([]);
        return;
      }

      const { data, error } = await supabase
        .from('accounts')
        .select('*')
        .eq('household_id', selectedHouseholdId)
        .eq('type', 'savings')
        .order('name');

      if (!error && data) {
        setAccounts(data);
      }
    };

    fetchAccounts();
  }, [selectedHouseholdId, supabase]);

  useEffect(() => {
    const handleOpenForm = () => {
      setShowForm(true);
      setEditingGoal(null);
      setFormData({ name: '', target_amount: '', deadline: '', account_id: '' });
    };

    window.addEventListener('open-goal-form', handleOpenForm);
    return () => {
      window.removeEventListener('open-goal-form', handleOpenForm);
    };
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedHouseholdId) return;

    try {
      if (editingGoal) {
        await updateGoal(editingGoal.id, {
          name: formData.name,
          target_amount: parseFloat(formData.target_amount),
          deadline: formData.deadline || null,
          account_id: formData.account_id || null,
        });
      } else {
        await createGoal({
          household_id: selectedHouseholdId,
          name: formData.name,
          target_amount: parseFloat(formData.target_amount),
          deadline: formData.deadline || undefined,
          account_id: formData.account_id || null,
        });
      }
      setShowForm(false);
      setEditingGoal(null);
      setFormData({ name: '', target_amount: '', deadline: '', account_id: '' });
    } catch (error) {
      console.error('Error saving goal:', error);
    }
  };

  const handleEdit = (goal: SavingsGoal) => {
    setEditingGoal(goal);
    setFormData({
      name: goal.name,
      target_amount: goal.target_amount.toString(),
      deadline: goal.deadline || '',
      account_id: goal.account_id || '',
    });
    setShowForm(true);
  };

  const handleDelete = async (goalId: string) => {
    if (confirm('Are you sure you want to delete this goal? This action cannot be undone.')) {
      try {
        await deleteGoal(goalId);
      } catch (error) {
        console.error('Error deleting goal:', error);
      }
    }
  };

  const handleAddContribution = async (goalId: string, amount: number) => {
    if (!userId || !selectedHouseholdId) return;

    try {
      await addContribution({
        goal_id: goalId,
        user_id: userId,
        amount,
        date: new Date().toISOString().split('T')[0],
      });
    } catch (error) {
      console.error('Error adding contribution:', error);
    }
  };

  const handleAllocate = (unallocated: UnallocatedSavings) => {
    setSelectedUnallocated(unallocated);
    setAllocateModalOpen(true);
  };

  const handleAllocateToGoal = async (goalId: string) => {
    if (!selectedUnallocated || !userId) return;

    try {
      await allocateSavings(selectedUnallocated.id, goalId, userId);
      setAllocateModalOpen(false);
      setSelectedUnallocated(null);
    } catch (error) {
      console.error('Error allocating savings:', error);
    }
  };

  // Drag and drop handlers
  const handleDragStart = (e: React.DragEvent, goalId: string) => {
    setDraggedGoalId(goalId);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', goalId);
  };

  const handleDragOver = (e: React.DragEvent, goalId: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    if (draggedGoalId && goalId !== draggedGoalId) {
      setDragOverGoalId(goalId);
    }
  };

  const handleDragLeave = () => {
    setDragOverGoalId(null);
  };

  const handleDrop = async (e: React.DragEvent, targetGoalId: string) => {
    e.preventDefault();
    setDragOverGoalId(null);

    if (!draggedGoalId || draggedGoalId === targetGoalId) {
      setDraggedGoalId(null);
      return;
    }

    // Reorder goals
    const draggedIndex = goals.findIndex((g) => g.id === draggedGoalId);
    const targetIndex = goals.findIndex((g) => g.id === targetGoalId);

    if (draggedIndex === -1 || targetIndex === -1) {
      setDraggedGoalId(null);
      return;
    }

    const newOrder = [...goals];
    const [removed] = newOrder.splice(draggedIndex, 1);
    newOrder.splice(targetIndex, 0, removed);

    try {
      await reorderGoals(newOrder.map((g) => g.id));
    } catch (error) {
      console.error('Error reordering goals:', error);
    }

    setDraggedGoalId(null);
  };

  const handleDragEnd = () => {
    setDraggedGoalId(null);
    setDragOverGoalId(null);
  };

  const getAccountById = (accountId: string | null) => {
    if (!accountId) return null;
    return accounts.find((a) => a.id === accountId) || null;
  };

  const stats = getSummaryStats();

  if (loading) {
    return (
      <MainLayout
        pageHeader={{
          title: 'Savings Goals',
          subtitle: 'Track your financial goals',
          secondary: (
            <div className="ml-auto">
              <Button onClick={() => setShowForm(true)} disabled={!selectedHouseholdId}>
                <Plus className="mr-2 h-4 w-4" />
                New Goal
              </Button>
            </div>
          ),
        }}
      >
        <div className="flex items-center justify-center py-24">
          <div className="text-muted-foreground">Loading goals...</div>
        </div>
      </MainLayout>
    );
  }

  return (
    <MainLayout
      pageHeader={{
        title: 'Savings Goals',
        subtitle: 'Track your financial goals',
        secondary: (
          <div className="ml-auto">
            <Button onClick={() => {
              setShowForm(true);
              setEditingGoal(null);
              setFormData({ name: '', target_amount: '', deadline: '', account_id: '' });
            }} disabled={!selectedHouseholdId}>
              <Plus className="mr-2 h-4 w-4" />
              New Goal
            </Button>
          </div>
        ),
      }}
    >
      <div className="space-y-6">
        {/* Summary Stats */}
        {goals.length > 0 && <SummaryStats stats={stats} />}

        {/* Goal Form Dialog */}
        <Dialog open={showForm} onOpenChange={setShowForm}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{editingGoal ? 'Edit Savings Goal' : 'Create Savings Goal'}</DialogTitle>
              <DialogDescription>
                {editingGoal
                  ? 'Update your savings goal details.'
                  : 'Set a new financial target to work towards.'}
              </DialogDescription>
            </DialogHeader>

            <form onSubmit={handleSubmit} className="space-y-4">
              <div>
                <label className="mb-1.5 block text-sm font-medium">Goal Name</label>
                <Input
                  type="text"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
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
                    value={formData.target_amount}
                    onChange={(e) => setFormData({ ...formData, target_amount: e.target.value })}
                    placeholder="0.00"
                    required
                  />
                </div>
                <div>
                  <label className="mb-1.5 block text-sm font-medium">Deadline (optional)</label>
                  <Input
                    type="date"
                    value={formData.deadline}
                    onChange={(e) => setFormData({ ...formData, deadline: e.target.value })}
                  />
                </div>
              </div>

              <div>
                <label className="mb-1.5 block text-sm font-medium">
                  Link to Savings Account (optional)
                </label>
                <Select
                  value={formData.account_id}
                  onChange={(e) => setFormData({ ...formData, account_id: e.target.value })}
                >
                  <option value="">No account linked</option>
                  {accounts.map((account) => (
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
                    setShowForm(false);
                    setEditingGoal(null);
                  }}
                >
                  Cancel
                </Button>
                <Button type="submit">{editingGoal ? 'Save Changes' : 'Create Goal'}</Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>

        {/* Allocate Modal */}
        <Dialog open={allocateModalOpen} onOpenChange={setAllocateModalOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Allocate Savings</DialogTitle>
              <DialogDescription>
                Choose which goal to assign{' '}
                {selectedUnallocated && formatCurrency(selectedUnallocated.amount)} to.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-2 max-h-[300px] overflow-y-auto">
              {goals.filter(g => getGoalProgress(g).percentage < 100).map((goal) => {
                const progress = getGoalProgress(goal);
                return (
                  <button
                    key={goal.id}
                    onClick={() => handleAllocateToGoal(goal.id)}
                    className="w-full p-3 rounded-lg border border-border hover:border-accent hover:bg-accent/5 transition-colors text-left"
                  >
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="font-medium">{goal.name}</p>
                        <p className="text-sm text-muted-foreground">
                          {formatCurrency(progress.current)} / {formatCurrency(goal.target_amount)}
                        </p>
                      </div>
                      <ProgressRing percentage={progress.percentage} size={48} strokeWidth={4} />
                    </div>
                  </button>
                );
              })}

              {goals.filter(g => getGoalProgress(g).percentage < 100).length === 0 && (
                <p className="text-center text-muted-foreground py-8">
                  All goals are complete! Create a new goal to allocate these savings.
                </p>
              )}
            </div>

            <DialogFooter>
              <Button variant="ghost" onClick={() => setAllocateModalOpen(false)}>
                Cancel
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Main Content */}
        {!selectedHouseholdId ? (
          <Card>
            <CardContent className="py-12 text-center">
              <Wallet className="h-12 w-12 mx-auto text-muted-foreground/50 mb-4" />
              <p className="text-muted-foreground">Please select or create a household first.</p>
            </CardContent>
          </Card>
        ) : goals.length === 0 && unallocatedSavings.length === 0 ? (
          <Card>
            <CardContent className="py-12 text-center">
              <Target className="h-12 w-12 mx-auto text-muted-foreground/50 mb-4" />
              <h3 className="text-lg font-medium mb-2">No savings goals yet</h3>
              <p className="text-muted-foreground mb-4">
                Create your first goal to start tracking your progress.
              </p>
              <Button onClick={() => setShowForm(true)}>
                <Plus className="mr-2 h-4 w-4" />
                Create Your First Goal
              </Button>
            </CardContent>
          </Card>
        ) : (
          <div className="grid gap-6 lg:grid-cols-3">
            {/* Goals List - Takes 2 columns on large screens */}
            <div className="lg:col-span-2 space-y-3">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                  Your Goals
                </h2>
                <p className="text-xs text-muted-foreground">
                  Drag to reorder priority
                </p>
              </div>

              {goals.map((goal) => {
                const progress = getGoalProgress(goal);
                const account = getAccountById(goal.account_id);

                return (
                  <div
                    key={goal.id}
                    draggable
                    onDragStart={(e) => handleDragStart(e, goal.id)}
                    onDragOver={(e) => handleDragOver(e, goal.id)}
                    onDragLeave={handleDragLeave}
                    onDrop={(e) => handleDrop(e, goal.id)}
                    onDragEnd={handleDragEnd}
                    className={`transition-transform ${
                      dragOverGoalId === goal.id ? 'translate-y-2' : ''
                    }`}
                  >
                    <GoalCard
                      goal={goal}
                      progress={progress}
                      account={account}
                      onEdit={() => handleEdit(goal)}
                      onDelete={() => handleDelete(goal.id)}
                      onAddContribution={(amount) => handleAddContribution(goal.id, amount)}
                      isDragging={draggedGoalId === goal.id}
                      dragHandleProps={{}}
                    />
                  </div>
                );
              })}
            </div>

            {/* Unallocated Savings Panel */}
            {unallocatedSavings.length > 0 && (
              <div className="lg:col-span-1">
                <Card className="border-amber-500/30 sticky top-4">
                  <CardHeader className="pb-3">
                    <div className="flex items-center gap-2">
                      <AlertCircle className="h-5 w-5 text-amber-500" />
                      <CardTitle className="text-base">Unallocated Savings</CardTitle>
                    </div>
                    <CardDescription>
                      {unallocatedSavings.length} deposit{unallocatedSavings.length !== 1 ? 's' : ''}{' '}
                      need{unallocatedSavings.length === 1 ? 's' : ''} to be assigned to goals
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-2">
                    {unallocatedSavings.slice(0, 5).map((item) => (
                      <UnallocatedItem
                        key={item.id}
                        item={item}
                        onAllocate={() => handleAllocate(item)}
                      />
                    ))}

                    {unallocatedSavings.length > 5 && (
                      <p className="text-xs text-center text-muted-foreground pt-2">
                        + {unallocatedSavings.length - 5} more
                      </p>
                    )}
                  </CardContent>
                </Card>
              </div>
            )}
          </div>
        )}
      </div>
    </MainLayout>
  );
}
