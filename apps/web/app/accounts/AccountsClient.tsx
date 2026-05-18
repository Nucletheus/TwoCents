'use client';

import { useEffect, useState } from 'react';
import { createClient } from '@/lib/supabase/client';
import MainLayout from '../components/MainLayout';
import Button from '../components/ui/Button';
import Input from '../components/ui/Input';
import Select from '../components/ui/Select';
import { useHousehold } from '../components/HouseholdProvider';

interface Household {
  id: string;
  name: string;
}

interface HouseholdMember {
  user_id: string;
  profiles: {
    name: string | null;
    email?: string | null;
  } | null;
}

interface Account {
  id: string;
  name: string;
  type: string;
  balance?: number | null;
  user_id?: string | null;
  household_id?: string | null;
}

const ACCOUNT_TYPES = [
  'Checking',
  'Savings',
  'Credit Card',
  'Loan',
  'Cash',
  'Investment',
  'Other',
];

export default function AccountsClient() {
  const supabase = createClient();
  const { selectedHouseholdId } = useHousehold();
  const [householdMembers, setHouseholdMembers] = useState<HouseholdMember[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [loading, setLoading] = useState(false);
  const [savingId, setSavingId] = useState<string | null>(null);

  useEffect(() => {
    if (selectedHouseholdId) {
      fetchHouseholdMembers();
      fetchAccounts();
    } else {
      setAccounts([]);
      setHouseholdMembers([]);
    }
  }, [selectedHouseholdId]);

  const fetchHouseholdMembers = async () => {
    if (!selectedHouseholdId) return;
    const { data, error } = await supabase
      .from('household_members')
      .select(`
        user_id,
        profiles ( id, name, email )
      `)
      .eq('household_id', selectedHouseholdId);
    if (error) {
      console.error('Error fetching household members', error);
      setHouseholdMembers([]);
      return;
    }
    const members =
      data?.map((m: any) => ({
        user_id: m.user_id,
        profiles: Array.isArray(m.profiles) ? m.profiles[0] : m.profiles,
      })) || [];
    setHouseholdMembers(members);
  };

  const fetchAccounts = async () => {
    setLoading(true);
    try {
      const { data, error } = await supabase
        .from('accounts')
        .select('id, name, type, balance, user_id, household_id')
        .eq('household_id', selectedHouseholdId)
        .order('name');
      if (error) throw error;
      setAccounts(data || []);
    } catch (error) {
      console.error('Error fetching accounts', error);
    } finally {
      setLoading(false);
    }
  };

  const createAccount = async () => {
    if (!selectedHouseholdId) return;
    const { data: { user } } = await supabase.auth.getUser();
    // Default owner to the current user to ensure the creator can manage the new account.
    const owner = user?.id || householdMembers[0]?.user_id || null;
    if (!owner) {
      alert('Failed to determine account owner');
      return;
    }
    const { data, error } = await supabase
      .from('accounts')
      .insert({
        name: 'New Account',
        type: 'Checking',
        balance: 0,
        user_id: owner,
        household_id: selectedHouseholdId,
      })
      .select()
      .single();
    if (error) {
      console.error('Error creating account', error);
      alert('Failed to create account');
      return;
    }
    setAccounts((prev) => [...prev, data]);
  };

  const updateAccount = async (id: string, patch: Partial<Account>) => {
    setSavingId(id);
    try {
      const { error } = await supabase.from('accounts').update(patch).eq('id', id);
      if (error) throw error;
      setAccounts((prev) =>
        prev.map((acc) => (acc.id === id ? { ...acc, ...patch } : acc))
      );
    } catch (error) {
      console.error('Error updating account', error);
      alert('Failed to update account');
    } finally {
      setSavingId(null);
    }
  };

  const deleteAccount = async (id: string) => {
    if (!confirm('Delete this account?')) return;
    setSavingId(id);
    try {
      const { error } = await supabase.from('accounts').delete().eq('id', id);
      if (error) throw error;
      setAccounts((prev) => prev.filter((acc) => acc.id !== id));
    } catch (error) {
      console.error('Error deleting account', error);
    } finally {
      setSavingId(null);
    }
  };

  return (
    <MainLayout
      pageHeader={{
        title: 'Accounts',
        subtitle: 'Manage partner accounts used across expenses and imports',
      }}
    >
      <div className="flex flex-col gap-4 h-full">
        <div className="flex items-center justify-between">
          <div className="text-sm text-muted-foreground">
            {loading ? 'Loading accounts…' : `${accounts.length} accounts`}
          </div>
          <Button onClick={createAccount} size="sm">
            Add Account
          </Button>
        </div>

        <div className="rounded-notion border border-border bg-card/80 backdrop-blur shadow-sm overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full border-collapse">
              <thead className="sticky top-0 bg-card border-b border-border">
                <tr className="text-left text-sm">
                  <th className="p-2 border-r border-border">Name</th>
                  <th className="p-2 border-r border-border">Type</th>
                  <th className="p-2 border-r border-border">Balance</th>
                  <th className="p-2 border-r border-border">Owner</th>
                  <th className="p-2 text-right">Actions</th>
                </tr>
              </thead>
              <tbody>
                {accounts.map((acc) => {
                  const owner = householdMembers.find((m) => m.user_id === acc.user_id);
                  return (
                    <tr key={acc.id} className="border-b border-border hover:bg-hover">
                      <td className="p-2 border-r border-border">
                        <Input
                          value={acc.name || ''}
                          onChange={(e) => updateAccount(acc.id, { name: e.target.value })}
                          className="h-9"
                        />
                      </td>
                      <td className="p-2 border-r border-border">
                        <Select
                          value={acc.type || 'Checking'}
                          onChange={(e) => updateAccount(acc.id, { type: e.target.value })}
                          className="h-9"
                        >
                          {ACCOUNT_TYPES.map((t) => (
                            <option key={t} value={t}>
                              {t}
                            </option>
                          ))}
                        </Select>
                      </td>
                      <td className="p-2 border-r border-border">
                        <Input
                          type="number"
                          step="0.01"
                          value={acc.balance ?? 0}
                          onChange={(e) =>
                            updateAccount(acc.id, { balance: parseFloat(e.target.value || '0') })
                          }
                          className="h-9"
                        />
                      </td>
                      <td className="p-2 border-r border-border">
                        <Select
                          value={acc.user_id || ''}
                          onChange={(e) => updateAccount(acc.id, { user_id: e.target.value || null })}
                          className="h-9"
                        >
                          <option value="">Unassigned</option>
                          {householdMembers.map((m) => (
                            <option key={m.user_id} value={m.user_id}>
                              {m.profiles?.name || 'Unnamed'}
                            </option>
                          ))}
                        </Select>
                      </td>
                      <td className="p-2 text-right">
                        <Button
                          variant="danger"
                          size="sm"
                          onClick={() => deleteAccount(acc.id)}
                          disabled={savingId === acc.id}
                        >
                          Delete
                        </Button>
                      </td>
                    </tr>
                  );
                })}
                {accounts.length === 0 && !loading && (
                  <tr>
                    <td className="p-4 text-center text-muted-foreground" colSpan={5}>
                      No accounts yet. Click “Add Account” to create one.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </MainLayout>
  );
}

