'use client';

import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
import { useExpenseSplits, calculateSettlements, formatCurrency } from '@twocents/shared';
import MainLayout from '../../../components/MainLayout';
import { Card, CardContent, CardHeader, CardTitle } from '../../../components/ui/Card';
import Input from '../../../components/ui/Input';
import Button from '../../../components/ui/Button';

export default function ExpenseSplitsClient({ expenseId }: { expenseId: string }) {
  const supabase = createClient();
  const { splits, loading, createSplits } = useExpenseSplits(supabase, expenseId);
  const [expense, setExpense] = useState<any>(null);
  const [householdMembers, setHouseholdMembers] = useState<any[]>([]);
  const [splitType, setSplitType] = useState<'equal' | 'percentage' | 'custom'>('equal');
  const [selectedUsers, setSelectedUsers] = useState<string[]>([]);
  const [percentages, setPercentages] = useState<Record<string, number>>({});
  const [customAmounts, setCustomAmounts] = useState<Record<string, number>>({});

  useEffect(() => {
    fetchExpense();
    fetchHouseholdMembers();
  }, [expenseId]);

  const fetchExpense = async () => {
    const { data, error } = await supabase
      .from('expenses')
      .select('*')
      .eq('id', expenseId)
      .single();

    if (!error && data) {
      setExpense(data);
    }
  };

  const fetchHouseholdMembers = async () => {
    if (!expense) return;

    const { data, error } = await supabase
      .from('household_members')
      .select(
        `
        user_id,
        profiles:user_id (
          id,
          name,
          email
        )
      `
      )
      .eq('household_id', expense.household_id);

    if (!error && data) {
      setHouseholdMembers(data);
    }
  };

  useEffect(() => {
    if (expense) {
      fetchHouseholdMembers();
    }
  }, [expense]);

  const handleSplit = async () => {
    if (!expense || selectedUsers.length === 0) return;

    try {
      let percentagesArray: number[] | undefined;
      let customAmountsArray: number[] | undefined;

      if (splitType === 'percentage') {
        percentagesArray = selectedUsers.map((userId) => percentages[userId] || 0);
        const total = percentagesArray.reduce((sum, pct) => sum + pct, 0);
        if (Math.abs(total - 100) > 0.01) {
          alert('Percentages must sum to 100%');
          return;
        }
      } else if (splitType === 'custom') {
        customAmountsArray = selectedUsers.map((userId) => customAmounts[userId] || 0);
        const total = customAmountsArray.reduce((sum, amt) => sum + amt, 0);
        if (Math.abs(total - expense.amount) > 0.01) {
          alert(`Custom amounts must sum to ${formatCurrency(expense.amount)}`);
          return;
        }
      }

      await createSplits(
        expenseId,
        splitType,
        selectedUsers,
        expense.amount,
        percentagesArray,
        customAmountsArray
      );
      alert('Expense split created successfully!');
    } catch (error: any) {
      alert('Error creating split: ' + error.message);
    }
  };

  return (
    <MainLayout
      pageHeader={{
        title: 'Split Expense',
        subtitle: expense ? `${expense.description || 'Expense'} • ${formatCurrency(expense.amount)}` : undefined,
        showHouseholdSelect: false,
      }}
    >
      {loading || !expense ? (
        <div className="flex items-center justify-center py-24">
          <div className="text-muted-foreground">Loading...</div>
        </div>
      ) : (
        <div className="mx-auto max-w-4xl space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Configure split</CardTitle>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="space-y-2">
                <label className="block text-sm font-medium">Split Type</label>
                <div className="flex flex-wrap gap-4">
                  <label className="flex items-center gap-2">
                    <input
                      type="radio"
                      value="equal"
                      checked={splitType === 'equal'}
                      onChange={(e) => setSplitType(e.target.value as any)}
                    />
                    Equal
                  </label>
                  <label className="flex items-center gap-2">
                    <input
                      type="radio"
                      value="percentage"
                      checked={splitType === 'percentage'}
                      onChange={(e) => setSplitType(e.target.value as any)}
                    />
                    Percentage
                  </label>
                  <label className="flex items-center gap-2">
                    <input
                      type="radio"
                      value="custom"
                      checked={splitType === 'custom'}
                      onChange={(e) => setSplitType(e.target.value as any)}
                    />
                    Custom Amounts
                  </label>
                </div>
              </div>

              <div className="space-y-2">
                <label className="block text-sm font-medium">Select Members</label>
                <div className="space-y-2">
                  {householdMembers.map((member: any) => (
                    <div key={member.user_id} className="flex items-center justify-between gap-3">
                      <label className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={selectedUsers.includes(member.user_id)}
                          onChange={(e) => {
                            if (e.target.checked) {
                              setSelectedUsers([...selectedUsers, member.user_id]);
                            } else {
                              setSelectedUsers(selectedUsers.filter((id) => id !== member.user_id));
                            }
                          }}
                        />
                        <span>{member.profiles?.name || member.profiles?.email || 'Unknown'}</span>
                      </label>

                      {selectedUsers.includes(member.user_id) && splitType === 'percentage' && (
                        <Input
                          type="number"
                          min="0"
                          max="100"
                          value={percentages[member.user_id] || 0}
                          onChange={(e) =>
                            setPercentages({
                              ...percentages,
                              [member.user_id]: parseFloat(e.target.value) || 0,
                            })
                          }
                          className="w-24"
                        />
                      )}

                      {selectedUsers.includes(member.user_id) && splitType === 'custom' && (
                        <Input
                          type="number"
                          min="0"
                          step="0.01"
                          value={customAmounts[member.user_id] || 0}
                          onChange={(e) =>
                            setCustomAmounts({
                              ...customAmounts,
                              [member.user_id]: parseFloat(e.target.value) || 0,
                            })
                          }
                          className="w-32"
                        />
                      )}
                    </div>
                  ))}
                </div>
              </div>

              <div className="flex justify-end">
                <Button onClick={handleSplit}>Create Split</Button>
              </div>
            </CardContent>
          </Card>

          {splits.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle>Current Splits</CardTitle>
              </CardHeader>
              <CardContent className="space-y-2">
                {splits.map((split: any) => (
                  <div key={split.user_id} className="flex items-center justify-between py-2">
                    <span>{split.profiles?.name || split.profiles?.email || 'Unknown'}</span>
                    <span className="font-medium">
                      {formatCurrency(split.amount)}
                      {split.percentage && ` (${split.percentage}%)`}
                    </span>
                  </div>
                ))}
              </CardContent>
            </Card>
          )}
        </div>
      )}
    </MainLayout>
  );
}

