'use client';

import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
import type { ColumnMapping } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import Select from '../../components/ui/Select';
import Input from '../../components/ui/Input';
import Button from '../../components/ui/Button';
import { ArrowRight, X, Edit2, Check, Plus } from 'lucide-react';

interface CSVTableMapperProps {
  headers: string[];
  rows: string[][];
  mapping: ColumnMapping;
  skipColumns: string[];
  householdId: string | null;
  onMappingChange: (mapping: ColumnMapping) => void;
  onSkipColumnsChange: (columns: string[]) => void;
  onHeadersChange?: (headers: string[]) => void;
  onNext: (payerId: string, householdId: string, accountId: string) => void;
}

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

export default function CSVTableMapper({
  headers,
  rows,
  mapping,
  skipColumns,
  householdId,
  onMappingChange,
  onSkipColumnsChange,
  onHeadersChange,
  onNext,
}: CSVTableMapperProps) {
  const supabase = createClient();
  const [householdMembers, setHouseholdMembers] = useState<HouseholdMember[]>([]);
  const [currentUser, setCurrentUser] = useState<{ id: string; name: string | null; email: string } | null>(null);
  const [households, setHouseholds] = useState<any[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [selectedPayer, setSelectedPayer] = useState<string>('');
  const [selectedHousehold, setSelectedHousehold] = useState<string>('');
  const [selectedAccount, setSelectedAccount] = useState<string>('');
  const [editingHeader, setEditingHeader] = useState<string | null>(null);
  const [headerEditValue, setHeaderEditValue] = useState<string>('');
  const [showNewHousehold, setShowNewHousehold] = useState(false);
  const [newHouseholdName, setNewHouseholdName] = useState('');
  const [showNewAccount, setShowNewAccount] = useState(false);
  const [newAccountName, setNewAccountName] = useState('');
  const [newAccountType, setNewAccountType] = useState<'chequing' | 'savings' | 'credit_card' | 'investment' | 'other'>('chequing');

  // Convert skipColumns to keepColumns (inverse logic)
  const keepColumns = headers.filter((h) => !skipColumns.includes(h));

  useEffect(() => {
    fetchCurrentUser();
    if (householdId) {
      fetchHouseholdMembers();
      setSelectedHousehold(householdId);
    }
    fetchHouseholds();
    fetchAccounts();
  }, [householdId]);

  const fetchCurrentUser = async () => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { data: profile } = await supabase
        .from('profiles')
        .select('name')
        .eq('id', user.id)
        .single();

      setCurrentUser({
        id: user.id,
        name: profile?.name || null,
        email: user.email || '',
      });
      if (!selectedPayer) {
        setSelectedPayer(user.id);
      }
    } catch (error) {
      console.error('Error fetching current user:', error);
    }
  };

  const fetchHouseholdMembers = async () => {
    if (!householdId) return;

    try {
      const { data, error } = await supabase
        .from('household_members')
        .select(
          `
          user_id,
          profiles:user_id (
            name,
            email
          )
        `
        )
        .eq('household_id', householdId);

      if (error) throw error;
      setHouseholdMembers(data || []);
    } catch (error) {
      console.error('Error fetching household members:', error);
    }
  };

  const fetchHouseholds = async () => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { data, error } = await supabase
        .from('household_members')
        .select(
          `
          household_id,
          households (
            id,
            name
          )
        `
        )
        .eq('user_id', user.id);

      if (error) throw error;
      const householdList = data?.map((m: any) => m.households).filter(Boolean) || [];
      setHouseholds(householdList);
      if (householdList.length > 0 && !selectedHousehold) {
        setSelectedHousehold(householdList[0].id);
      }
    } catch (error) {
      console.error('Error fetching households:', error);
    }
  };

  const fetchAccounts = async () => {
    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { data, error } = await supabase
        .from('accounts')
        .select('id, name, type')
        .eq('user_id', user.id)
        .order('name');

      if (error) throw error;
      setAccounts(data || []);
    } catch (error) {
      console.error('Error fetching accounts:', error);
    }
  };

  const createHousehold = async () => {
    if (!newHouseholdName.trim()) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { data, error } = await supabase.rpc('create_household', {
        p_name: newHouseholdName.trim(),
      });

      if (error) throw error;
      await fetchHouseholds();
      if (data) {
        setSelectedHousehold(data.id);
      }
      setShowNewHousehold(false);
      setNewHouseholdName('');
    } catch (error) {
      console.error('Error creating household:', error);
      alert('Failed to create household');
    }
  };

  const createAccount = async () => {
    if (!newAccountName.trim()) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { data, error } = await supabase
        .from('accounts')
        .insert({
          user_id: user.id,
          household_id: selectedHousehold || null,
          name: newAccountName.trim(),
          type: newAccountType,
        })
        .select()
        .single();

      if (error) throw error;
      await fetchAccounts();
      if (data) {
        setSelectedAccount(data.id);
      }
      setShowNewAccount(false);
      setNewAccountName('');
    } catch (error) {
      console.error('Error creating account:', error);
      alert('Failed to create account');
    }
  };

  const updateMapping = (field: keyof ColumnMapping, column: string) => {
    onMappingChange({ ...mapping, [field]: column || undefined });
  };

  const toggleKeepColumn = (column: string) => {
    // Inverse logic: if column is in keepColumns (not skipped), remove it (skip it)
    // If column is skipped, add it back (keep it)
    if (keepColumns.includes(column)) {
      // Currently kept, so skip it
      onSkipColumnsChange([...skipColumns, column]);
      // Remove from mapping if it was mapped
      const newMapping = { ...mapping };
      Object.keys(newMapping).forEach((key) => {
        if (newMapping[key as keyof ColumnMapping] === column) {
          delete newMapping[key as keyof ColumnMapping];
        }
      });
      onMappingChange(newMapping);
    } else {
      // Currently skipped, so keep it
      onSkipColumnsChange(skipColumns.filter((c) => c !== column));
    }
  };

  const handleHeaderEdit = (header: string) => {
    setEditingHeader(header);
    setHeaderEditValue(header);
  };

  const handleHeaderSave = () => {
    if (!editingHeader || !headerEditValue.trim()) return;

    const newHeaders = [...headers];
    const index = newHeaders.indexOf(editingHeader);
    if (index >= 0) {
      newHeaders[index] = headerEditValue.trim();

      const newMapping = { ...mapping };
      Object.keys(newMapping).forEach((key) => {
        if (newMapping[key as keyof ColumnMapping] === editingHeader) {
          newMapping[key as keyof ColumnMapping] = headerEditValue.trim();
        }
      });
      onMappingChange(newMapping);

      if (skipColumns.includes(editingHeader)) {
        const newSkipColumns = skipColumns.map((col) =>
          col === editingHeader ? headerEditValue.trim() : col
        );
        onSkipColumnsChange(newSkipColumns);
      }

      onHeadersChange?.(newHeaders);
    }

    setEditingHeader(null);
  };

  const availableColumns = keepColumns;
  const visibleRows = rows.slice(0, 50);

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Map CSV Columns</CardTitle>
          <CardDescription>Edit column headers and select which columns to keep</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4 pb-24">
          {/* CSV Table Preview */}
          <div className="overflow-x-auto">
            <table className="w-full border-collapse border border-border">
              <thead className="bg-muted">
                <tr>
                  {headers.map((header) => {
                    const isKept = keepColumns.includes(header);
                    const isMapped = Object.values(mapping).some((v) =>
                      v === header || (Array.isArray(v) && v.includes(header))
                    );
                    const isEditing = editingHeader === header;

                    return (
                      <th
                        key={header}
                        className={`border border-border p-2 text-left text-sm font-medium ${
                          !isKept ? 'opacity-50 bg-red-50 dark:bg-red-950' : ''
                        } ${isMapped ? 'bg-green-50 dark:bg-green-950' : ''}`}
                      >
                        <div className="flex items-center gap-2">
                          {isEditing ? (
                            <div className="flex items-center gap-1 flex-1">
                              <Input
                                value={headerEditValue}
                                onChange={(e) => setHeaderEditValue(e.target.value)}
                                className="h-7 text-sm flex-1"
                                onKeyDown={(e) => {
                                  if (e.key === 'Enter') {
                                    handleHeaderSave();
                                  }
                                  if (e.key === 'Escape') {
                                    setEditingHeader(null);
                                  }
                                }}
                                autoFocus
                              />
                              <button
                                onClick={handleHeaderSave}
                                className="p-1 hover:bg-hover rounded"
                              >
                                <Check className="h-3 w-3" />
                              </button>
                              <button
                                onClick={() => setEditingHeader(null)}
                                className="p-1 hover:bg-hover rounded"
                              >
                                <X className="h-3 w-3" />
                              </button>
                            </div>
                          ) : (
                            <>
                              <span className="flex-1">{header}</span>
                              <div className="flex items-center gap-1">
                                <button
                                  onClick={() => handleHeaderEdit(header)}
                                  className="p-1 hover:bg-hover rounded"
                                  title="Edit header"
                                >
                                  <Edit2 className="h-3 w-3" />
                                </button>
                                <label className="cursor-pointer">
                                  <input
                                    type="checkbox"
                                    checked={isKept}
                                    onChange={() => toggleKeepColumn(header)}
                                    className="rounded border-border"
                                    title="Keep column"
                                  />
                                </label>
                              </div>
                            </>
                          )}
                        </div>
                      </th>
                    );
                  })}
                </tr>
              </thead>
              <tbody>
                {visibleRows.map((row, rowIndex) => (
                  <tr key={rowIndex} className="border-b border-border hover:bg-hover">
                    {headers.map((header, colIndex) => {
                      const isKept = keepColumns.includes(header);
                      return (
                        <td
                          key={colIndex}
                          className={`border-r border-border p-2 text-sm ${
                            !isKept ? 'opacity-50' : ''
                          }`}
                        >
                          {row[colIndex] || '-'}
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
            {rows.length > 50 && (
              <p className="mt-2 text-xs text-muted-foreground text-center">
                Showing first 50 of {rows.length} rows
              </p>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Fixed Glass Action Bar at Bottom */}
      <div className="fixed bottom-0 left-0 right-0 z-50 backdrop-blur-md bg-background/80 border-t border-border shadow-lg">
        <div className="mx-auto max-w-7xl px-4 py-3">
          <div className="flex flex-wrap items-end gap-4">
            <div className="flex-1 min-w-[180px]">
              <label className="mb-1 block text-xs font-medium text-muted-foreground">Partner</label>
              <Select
                value={selectedPayer}
                onChange={(e) => setSelectedPayer(e.target.value)}
                className="w-full"
              >
                <option value="">Select partner...</option>
                {currentUser && (
                  <option value={currentUser.id}>
                    {currentUser.name || currentUser.email} (Me)
                  </option>
                )}
                {householdMembers
                  .filter((m) => m.user_id !== currentUser?.id)
                  .map((member) => (
                    <option key={member.user_id} value={member.user_id}>
                      {member.profiles?.name || member.profiles?.email || 'Unknown'}
                    </option>
                  ))}
              </Select>
            </div>

            <div className="flex-1 min-w-[180px]">
              <label className="mb-1 block text-xs font-medium text-muted-foreground">Household</label>
              <Select
                value={selectedHousehold}
                onChange={(e) => {
                  setSelectedHousehold(e.target.value);
                  if (e.target.value === 'new') {
                    setShowNewHousehold(true);
                  } else {
                    setShowNewHousehold(false);
                  }
                }}
                className="w-full"
              >
                <option value="">Select household...</option>
                {households.map((household) => (
                  <option key={household.id} value={household.id}>
                    {household.name}
                  </option>
                ))}
                <option value="new">+ Create New Household</option>
              </Select>
              {showNewHousehold && (
                <div className="mt-2 space-y-2 p-3 rounded-md border border-border bg-card">
                  <Input
                    value={newHouseholdName}
                    onChange={(e) => setNewHouseholdName(e.target.value)}
                    placeholder="Household name"
                    className="w-full"
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        createHousehold();
                      }
                      if (e.key === 'Escape') {
                        setShowNewHousehold(false);
                        setNewHouseholdName('');
                      }
                    }}
                    autoFocus
                  />
                  <div className="flex gap-2">
                    <Button size="sm" onClick={createHousehold} className="flex-1">
                      <Check className="mr-1 h-3 w-3" />
                      Create
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => {
                        setShowNewHousehold(false);
                        setNewHouseholdName('');
                      }}
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                </div>
              )}
            </div>

            <div className="flex-1 min-w-[180px]">
              <label className="mb-1 block text-xs font-medium text-muted-foreground">Account</label>
              <Select
                value={selectedAccount}
                onChange={(e) => {
                  setSelectedAccount(e.target.value);
                  if (e.target.value === 'new') {
                    setShowNewAccount(true);
                  } else {
                    setShowNewAccount(false);
                  }
                }}
                className="w-full"
              >
                <option value="">Select account...</option>
                {accounts.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name} ({account.type})
                  </option>
                ))}
                <option value="new">+ Create New Account</option>
              </Select>
              {showNewAccount && (
                <div className="mt-2 space-y-2 p-3 rounded-md border border-border bg-card">
                  <Input
                    value={newAccountName}
                    onChange={(e) => setNewAccountName(e.target.value)}
                    placeholder="Account name"
                    className="w-full"
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        createAccount();
                      }
                      if (e.key === 'Escape') {
                        setShowNewAccount(false);
                        setNewAccountName('');
                      }
                    }}
                    autoFocus
                  />
                  <Select
                    value={newAccountType}
                    onChange={(e) =>
                      setNewAccountType(
                        e.target.value as 'chequing' | 'savings' | 'credit_card' | 'investment' | 'other'
                      )
                    }
                    className="w-full"
                  >
                    <option value="chequing">Chequing</option>
                    <option value="savings">Savings</option>
                    <option value="credit_card">Credit Card</option>
                    <option value="investment">Investment</option>
                    <option value="other">Other</option>
                  </Select>
                  <div className="flex gap-2">
                    <Button size="sm" onClick={createAccount} className="flex-1">
                      <Check className="mr-1 h-3 w-3" />
                      Create
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => {
                        setShowNewAccount(false);
                        setNewAccountName('');
                      }}
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                </div>
              )}
            </div>

            <div className="flex-shrink-0">
              <Button
                onClick={() => onNext(selectedPayer, selectedHousehold, selectedAccount)}
                disabled={
                  !mapping.date ||
                  !(Array.isArray(mapping.amount) ? mapping.amount.length > 0 : Boolean(mapping.amount)) ||
                  !selectedPayer ||
                  !selectedHousehold ||
                  !selectedAccount
                }
                className="mt-6"
              >
                Next: Preview
                <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
