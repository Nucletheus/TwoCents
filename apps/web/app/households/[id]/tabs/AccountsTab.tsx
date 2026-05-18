'use client';

import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { SupabaseClient } from '@supabase/supabase-js';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../../components/ui/Card';
import Input from '../../../components/ui/Input';
import Button from '../../../components/ui/Button';
import Select from '../../../components/ui/Select';
import { Plus, Trash2, CreditCard, Wallet } from 'lucide-react';

interface Account {
  id: string;
  name: string;
  type: string;
  balance?: number | null;
  user_id?: string | null;
  household_id?: string | null;
}

interface HouseholdMember {
  user_id: string;
  profiles: {
    name: string | null;
  } | null;
}

interface AccountsTabProps {
  householdId: string;
  members: HouseholdMember[];
  supabase: SupabaseClient;
}

const ACCOUNT_TYPES = [
  { value: 'chequing', label: 'Chequing' },
  { value: 'savings', label: 'Savings' },
  { value: 'credit_card', label: 'Credit Card' },
  { value: 'investment', label: 'Investment' },
  { value: 'other', label: 'Other' },
];

// Individual account row component to manage its own local state
function AccountRow({
  account,
  members,
  supabase,
  onDelete,
  onUpdated,
}: {
  account: Account;
  members: HouseholdMember[];
  supabase: SupabaseClient;
  onDelete: (id: string) => void;
  onUpdated: (id: string, updates: Partial<Account>) => void;
}) {
  // Local state for editing - prevents lag by not hitting DB on every keystroke
  const [localName, setLocalName] = useState(account.name || '');
  const [localBalance, setLocalBalance] = useState(String(account.balance ?? 0));
  const [saving, setSaving] = useState(false);
  const pendingUpdate = useRef<Partial<Account>>({});

  // Sync local state when account prop changes (e.g., after refetch)
  useEffect(() => {
    setLocalName(account.name || '');
    setLocalBalance(String(account.balance ?? 0));
  }, [account.name, account.balance]);

  const saveToDb = useCallback(async (patch: Partial<Account>) => {
    if (Object.keys(patch).length === 0) return;
    
    setSaving(true);
    try {
      const { error } = await supabase.from('accounts').update(patch).eq('id', account.id);
      if (error) throw error;
      onUpdated(account.id, patch);
    } catch (error) {
      console.error('Error updating account:', error);
      alert('Failed to update account');
    } finally {
      setSaving(false);
    }
  }, [supabase, account.id, onUpdated]);

  const handleNameBlur = () => {
    if (localName !== account.name) {
      saveToDb({ name: localName });
    }
  };

  const handleBalanceBlur = () => {
    const numValue = parseFloat(localBalance || '0');
    if (numValue !== account.balance) {
      saveToDb({ balance: numValue });
    }
  };

  const handleTypeChange = (newType: string) => {
    saveToDb({ type: newType });
  };

  const handleOwnerChange = (newOwner: string) => {
    saveToDb({ user_id: newOwner || null });
  };

  const getAccountIcon = (type: string) => {
    if (type === 'credit_card') return <CreditCard className="h-4 w-4" />;
    return <Wallet className="h-4 w-4" />;
  };

  return (
    <tr className="border-b border-border last:border-b-0 hover:bg-hover/50">
      <td className="p-3">
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground">{getAccountIcon(account.type)}</span>
          <Input
            value={localName}
            onChange={(e) => setLocalName(e.target.value)}
            onBlur={handleNameBlur}
            disabled={saving}
            className="h-9 max-w-[200px]"
          />
        </div>
      </td>
      <td className="p-3">
        <Select
          value={account.type || 'chequing'}
          onChange={(e) => handleTypeChange(e.target.value)}
          disabled={saving}
          className="h-9"
        >
          {ACCOUNT_TYPES.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </Select>
      </td>
      <td className="p-3">
        <Input
          type="number"
          step="0.01"
          value={localBalance}
          onChange={(e) => setLocalBalance(e.target.value)}
          onBlur={handleBalanceBlur}
          disabled={saving}
          className="h-9 w-32"
        />
      </td>
      <td className="p-3">
        <Select
          value={account.user_id || ''}
          onChange={(e) => handleOwnerChange(e.target.value)}
          disabled={saving}
          className="h-9"
        >
          <option value="">Unassigned</option>
          {members.map((m) => (
            <option key={m.user_id} value={m.user_id}>
              {m.profiles?.name || 'Unnamed'}
            </option>
          ))}
        </Select>
      </td>
      <td className="p-3 text-right">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onDelete(account.id)}
          disabled={saving}
          className="text-muted-foreground hover:text-red-600"
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </td>
    </tr>
  );
}

export default function AccountsTab({ householdId, members, supabase }: AccountsTabProps) {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [loading, setLoading] = useState(true);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  const membersSorted = useMemo(() => {
    return [...members].sort((a, b) => {
      const aName = (a.profiles?.name || 'Unnamed').toLowerCase();
      const bName = (b.profiles?.name || 'Unnamed').toLowerCase();
      return aName.localeCompare(bName);
    });
  }, [members]);

  const groupedAccounts = useMemo(() => {
    const map = new Map<string, Account[]>();
    for (const acc of accounts) {
      const key = acc.user_id ? String(acc.user_id) : '__unassigned__';
      const arr = map.get(key) || [];
      arr.push(acc);
      map.set(key, arr);
    }
    for (const arr of map.values()) {
      arr.sort((a, b) => (a.name || '').localeCompare(b.name || ''));
    }
    return map;
  }, [accounts]);

  const sections = useMemo(() => {
    const list: Array<{ key: string; label: string; accounts: Account[] }> = [];

    for (const m of membersSorted) {
      const key = String(m.user_id);
      const label = m.profiles?.name || 'Unnamed';
      list.push({ key, label, accounts: groupedAccounts.get(key) || [] });
    }

    list.push({
      key: '__unassigned__',
      label: 'Unassigned',
      accounts: groupedAccounts.get('__unassigned__') || [],
    });

    return list;
  }, [groupedAccounts, membersSorted]);

  useEffect(() => {
    fetchAccounts();
  }, [householdId]);

  const fetchAccounts = async () => {
    setLoading(true);
    try {
      const { data, error } = await supabase
        .from('accounts')
        .select('id, name, type, balance, user_id, household_id')
        .eq('household_id', householdId)
        .order('name');

      if (error) throw error;
      setAccounts(data || []);
    } catch (error) {
      console.error('Error fetching accounts:', error);
    } finally {
      setLoading(false);
    }
  };

  const createAccount = async () => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      // Default owner to the current user to avoid creating an account that the creator can't manage
      // (and to keep behavior consistent across screens).
      const owner = user?.id || members[0]?.user_id || null;
      if (!owner) {
        alert('Failed to determine account owner');
        return;
      }

      const { data, error } = await supabase
        .from('accounts')
        .insert({
          name: 'New Account',
          type: 'chequing',
          balance: 0,
          user_id: owner,
          household_id: householdId,
        })
        .select()
        .single();

      if (error) throw error;
      setAccounts((prev) => [...prev, data]);
    } catch (error) {
      console.error('Error creating account:', error);
      alert('Failed to create account');
    }
  };

  const handleAccountUpdated = useCallback((id: string, updates: Partial<Account>) => {
    setAccounts((prev) => prev.map((acc) => (acc.id === id ? { ...acc, ...updates } : acc)));
  }, []);

  const deleteAccount = async () => {
    if (!confirmDeleteId) return;

    setDeleting(true);
    try {
      const { error } = await supabase.from('accounts').delete().eq('id', confirmDeleteId);
      if (error) throw error;
      setAccounts((prev) => prev.filter((acc) => acc.id !== confirmDeleteId));
      setConfirmDeleteId(null);
    } catch (error) {
      console.error('Error deleting account:', error);
      alert('Failed to delete account');
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Wallet className="h-5 w-5" />
                Accounts
              </CardTitle>
              <CardDescription>
                {loading ? 'Loading...' : `${accounts.length} ${accounts.length === 1 ? 'account' : 'accounts'}`}
              </CardDescription>
            </div>
            <Button onClick={createAccount} size="sm">
              <Plus className="mr-2 h-4 w-4" />
              Add Account
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="py-8 text-center text-muted-foreground">Loading accounts...</div>
          ) : accounts.length === 0 ? (
            <div className="py-8 text-center text-muted-foreground">
              No accounts yet. Click "Add Account" to create one.
            </div>
          ) : (
            <div className="space-y-6">
              {sections.map((section) => (
                <div key={section.key} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="text-sm font-semibold">{section.label}</div>
                    <div className="text-xs text-muted-foreground">
                      {section.accounts.length} {section.accounts.length === 1 ? 'account' : 'accounts'}
                    </div>
                  </div>

                  {section.accounts.length === 0 ? (
                    <div className="rounded-notion border border-dashed border-border bg-muted/20 p-4 text-sm text-muted-foreground">
                      No accounts
                    </div>
                  ) : (
                    <div className="rounded-notion border border-border overflow-hidden">
                      <div className="overflow-x-auto">
                        <table className="w-full border-collapse">
                          <thead className="bg-muted/50">
                            <tr className="text-left text-sm font-medium text-muted-foreground">
                              <th className="p-3 border-b border-border">Name</th>
                              <th className="p-3 border-b border-border">Type</th>
                              <th className="p-3 border-b border-border">Balance</th>
                              <th className="p-3 border-b border-border">Owner</th>
                              <th className="p-3 border-b border-border text-right">Actions</th>
                            </tr>
                          </thead>
                          <tbody>
                            {section.accounts.map((acc) => (
                              <AccountRow
                                key={acc.id}
                                account={acc}
                                members={membersSorted}
                                supabase={supabase}
                                onDelete={setConfirmDeleteId}
                                onUpdated={handleAccountUpdated}
                              />
                            ))}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Delete Confirmation Dialog */}
      {confirmDeleteId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <Card className="w-full max-w-md">
            <CardHeader>
              <CardTitle>Delete Account</CardTitle>
              <CardDescription>
                Are you sure you want to delete this account? This action cannot be undone.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex gap-2 justify-end">
                <Button variant="ghost" onClick={() => setConfirmDeleteId(null)} disabled={deleting}>
                  Cancel
                </Button>
                <Button variant="danger" onClick={deleteAccount} disabled={deleting}>
                  {deleting ? 'Deleting...' : 'Delete'}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
