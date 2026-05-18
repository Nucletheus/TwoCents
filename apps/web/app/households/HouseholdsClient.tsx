'use client';

import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
import { useHouseholds, formatCurrency } from '@twocents/shared';
import MainLayout from '../components/MainLayout';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Input from '../components/ui/Input';
import Button from '../components/ui/Button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../components/ui/dialog';
import { Plus, Users, Trash2, Home } from 'lucide-react';
import Link from 'next/link';
import { useHousehold } from '../components/HouseholdProvider';

// Household Card Component
function HouseholdCard({
  household,
  onDelete,
  isDeleting,
}: {
  household: { id: string; name: string; created_at: string };
  onDelete: () => void;
  isDeleting: boolean;
}) {
  const supabase = createClient();
  const [members, setMembers] = useState<Array<{ user_id: string; profiles: { name: string | null } | null }>>([]);
  const [userColors, setUserColors] = useState<Record<string, string>>({});
  const [netSavings, setNetSavings] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchCardData = async () => {
      try {
        // Fetch household members
        const { data: membersData, error: membersError } = await supabase
          .from('household_members')
          .select('user_id')
          .eq('household_id', household.id);

        if (membersError) throw membersError;

        const userIds = membersData?.map((m: any) => m.user_id) || [];
        let profilesMap: Record<string, { name: string | null }> = {};

        if (userIds.length > 0) {
          const { data: profilesData } = await supabase
            .from('profiles')
            .select('id, name')
            .in('id', userIds);

          if (profilesData) {
            profilesMap = Object.fromEntries(
              profilesData.map((p: any) => [p.id, { name: p.name }])
            );
          }
        }

        const enrichedMembers = (membersData || []).map((m: any) => ({
          user_id: m.user_id,
          profiles: profilesMap[m.user_id] || null,
        }));

        setMembers(enrichedMembers);

        // Fetch user colors
        const { data: colorsData, error: colorsError } = await supabase
          .from('household_member_colors')
          .select('user_id, color')
          .eq('household_id', household.id);

        if (colorsError) throw colorsError;

        const colorsMap: Record<string, string> = {};
        (colorsData || []).forEach((item: any) => {
          colorsMap[item.user_id] = item.color;
        });
        setUserColors(colorsMap);

        // Fetch accounts and calculate net savings
        const { data: accountsData, error: accountsError } = await supabase
          .from('accounts')
          .select('type, balance')
          .eq('household_id', household.id);

        if (accountsError) throw accountsError;

        const savings = (accountsData || [])
          .filter((acc: any) => acc.type !== 'credit_card')
          .reduce((sum: number, acc: any) => sum + (Number(acc.balance) || 0), 0);

        setNetSavings(savings);
      } catch (error) {
        console.error('Error fetching household card data:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchCardData();
  }, [household.id, supabase]);

  return (
    <Link href={`/households/${household.id}`} className="block">
      <Card className="group relative overflow-hidden transition-all duration-200 hover:shadow-lg hover:border-accent/30 cursor-pointer">
        <CardContent className="px-2.5 py-1.5 my-0">
        <div className="flex items-start gap-4">
          {/* Icon */}
          <div className="flex-shrink-0 p-3 rounded-xl bg-gradient-to-br from-accent/10 to-blue-500/10">
            <Home className="h-6 w-6 text-accent" />
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2">
              <div>
                <h3 className="font-semibold text-lg truncate">{household.name}</h3>
                <p className="text-sm text-muted-foreground mt-0.5">
                  Created {new Date(household.created_at).toLocaleDateString('en-US', {
                    month: 'short',
                    day: 'numeric',
                    year: 'numeric',
                  })}
                </p>
              </div>
              <button
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  onDelete();
                }}
                disabled={isDeleting}
                className="opacity-0 group-hover:opacity-100 rounded-lg p-2 text-muted-foreground hover:bg-red-500/10 hover:text-red-500 transition-all disabled:opacity-50"
                aria-label="Delete household"
              >
                <Trash2 className="h-4 w-4" />
              </button>
            </div>

            {/* Members and Net Savings */}
            {!loading && (
              <div className="mt-4 space-y-3">
                {/* Household Members */}
                {members.length > 0 && (
                  <div className="flex flex-wrap items-center gap-2">
                    {members.map((member) => {
                      const memberName = member.profiles?.name || 'Unknown';
                      const memberColor = userColors[member.user_id] || '#6366f1'; // Default to accent color
                      return (
                        <span
                          key={member.user_id}
                          className="inline-flex items-center px-2.5 py-1 rounded-md text-xs font-medium text-white"
                          style={{ backgroundColor: memberColor }}
                        >
                          {memberName}
                        </span>
                      );
                    })}
                  </div>
                )}

                {/* Net Savings */}
                {netSavings !== null && (
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-muted-foreground">Net Savings:</span>
                    <span className="text-sm font-semibold">
                      {formatCurrency(netSavings)}
                    </span>
                  </div>
                )}
              </div>
            )}

          </div>
        </div>
      </CardContent>
    </Card>
    </Link>
  );
}

// Empty State Component
function EmptyState({ onCreateClick }: { onCreateClick: () => void }) {
  return (
    <Card className="col-span-full">
      <CardContent className="py-16 text-center">
        <div className="mx-auto w-16 h-16 rounded-2xl bg-accent/10 flex items-center justify-center mb-4">
          <Home className="h-8 w-8 text-accent" />
        </div>
        <h3 className="text-lg font-semibold mb-2">No households yet</h3>
        <p className="text-muted-foreground mb-6 max-w-sm mx-auto">
          Create your first household to start tracking expenses with your partner.
        </p>
        <Button onClick={onCreateClick}>
          <Plus className="mr-2 h-4 w-4" />
          Create Household
        </Button>
      </CardContent>
    </Card>
  );
}

export default function HouseholdsClient() {
  const supabase = createClient();
  const { households, loading, createHousehold, deleteHousehold, error } = useHouseholds(supabase);
  const { refetch: refetchGlobalHouseholds } = useHousehold();
  const [newHouseholdName, setNewHouseholdName] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  const handleCreateHousehold = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newHouseholdName.trim()) return;

    setIsCreating(true);
    try {
      await createHousehold(newHouseholdName.trim());
      setNewHouseholdName('');
      setShowCreateDialog(false);
      refetchGlobalHouseholds();
    } catch (err) {
      console.error('Error creating household:', err);
    } finally {
      setIsCreating(false);
    }
  };

  const handleDeleteClick = (householdId: string) => {
    setConfirmDeleteId(householdId);
  };

  const handleConfirmDelete = async () => {
    if (!confirmDeleteId) return;

    setDeletingId(confirmDeleteId);
    try {
      await deleteHousehold(confirmDeleteId);
      setConfirmDeleteId(null);
      refetchGlobalHouseholds();
    } catch (err) {
      console.error('Error deleting household:', err);
    } finally {
      setDeletingId(null);
    }
  };

  const handleCancelDelete = () => {
    setConfirmDeleteId(null);
  };

  if (loading) {
    return (
      <MainLayout
        pageHeader={{
          title: 'Households',
          subtitle: 'Manage your households and members',
        }}
      >
        <div className="flex items-center justify-center py-24">
          <div className="text-muted-foreground">Loading households...</div>
        </div>
      </MainLayout>
    );
  }

  return (
    <MainLayout
      pageHeader={{
        title: 'Households',
        subtitle: 'Manage your households and members',
      }}
    >
      <div className="max-w-4xl mx-auto">
        {error && (
          <Card variant="outlined" className="border-red-500 mb-6">
            <CardContent className="py-4">
              <p className="text-sm text-red-600">{error.message}</p>
            </CardContent>
          </Card>
        )}

        {/* Households Grid */}
        <div className="grid gap-4 sm:grid-cols-2">
          {households.length === 0 ? (
            <EmptyState onCreateClick={() => setShowCreateDialog(true)} />
          ) : (
            households.map((household) => (
              <HouseholdCard
                key={household.id}
                household={household}
                onDelete={() => handleDeleteClick(household.id)}
                isDeleting={deletingId === household.id}
              />
            ))
          )}
        </div>

        {/* Create Household Dialog */}
        <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create New Household</DialogTitle>
              <DialogDescription>
                Add a new household to start tracking expenses together.
              </DialogDescription>
            </DialogHeader>
            <form onSubmit={handleCreateHousehold}>
              <div className="py-4">
                <label className="block text-sm font-medium mb-2">
                  Household Name
                </label>
                <Input
                  type="text"
                  value={newHouseholdName}
                  onChange={(e) => setNewHouseholdName(e.target.value)}
                  placeholder="e.g., Smith Family, Our Home"
                  disabled={isCreating}
                  autoFocus
                />
              </div>
              <DialogFooter>
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => setShowCreateDialog(false)}
                  disabled={isCreating}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={isCreating || !newHouseholdName.trim()}>
                  {isCreating ? 'Creating...' : 'Create Household'}
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>

        {/* Delete Confirmation Dialog */}
        <Dialog open={!!confirmDeleteId} onOpenChange={() => setConfirmDeleteId(null)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Delete Household</DialogTitle>
              <DialogDescription>
                Are you sure you want to delete this household? This action cannot be undone and will delete all associated expenses, goals, and data.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="ghost" onClick={handleCancelDelete} disabled={deletingId !== null}>
                Cancel
              </Button>
              <Button variant="danger" onClick={handleConfirmDelete} disabled={deletingId !== null}>
                {deletingId ? 'Deleting...' : 'Delete'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </MainLayout>
  );
}
