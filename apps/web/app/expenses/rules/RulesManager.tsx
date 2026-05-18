'use client';

import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
import type { TransactionRule, Category } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import Button from '../../components/ui/Button';
import RuleForm from './RuleForm';
import { Plus, Edit2, Trash2, ToggleLeft, ToggleRight } from 'lucide-react';

interface RulesManagerProps {
  householdId: string | null;
  categories: Category[];
}

export default function RulesManager({ householdId, categories }: RulesManagerProps) {
  const supabase = createClient();
  const [rules, setRules] = useState<TransactionRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [editingRule, setEditingRule] = useState<TransactionRule | null>(null);

  useEffect(() => {
    if (householdId) {
      fetchRules();
    }
  }, [householdId]);

  const fetchRules = async () => {
    if (!householdId) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { data, error } = await supabase
        .from('transaction_rules')
        .select('*')
        .eq('household_id', householdId)
        .order('priority', { ascending: false });

      if (error) throw error;
      setRules(data || []);
    } catch (error) {
      console.error('Error fetching rules:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleCreate = async (ruleData: Omit<TransactionRule, 'id' | 'created_at'>) => {
    if (!householdId) return;

    try {
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) return;

      const { error } = await supabase.from('transaction_rules').insert({
        ...ruleData,
        household_id: householdId,
        user_id: user.id,
      });

      if (error) throw error;
      await fetchRules();
      setShowForm(false);
      setEditingRule(null);
    } catch (error) {
      console.error('Error creating rule:', error);
      throw error;
    }
  };

  const handleUpdate = async (ruleData: Omit<TransactionRule, 'id' | 'created_at'>) => {
    if (!editingRule) return;

    try {
      const { error } = await supabase
        .from('transaction_rules')
        .update(ruleData)
        .eq('id', editingRule.id);

      if (error) throw error;
      await fetchRules();
      setShowForm(false);
      setEditingRule(null);
    } catch (error) {
      console.error('Error updating rule:', error);
      throw error;
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Are you sure you want to delete this rule?')) return;

    try {
      const { error } = await supabase.from('transaction_rules').delete().eq('id', id);
      if (error) throw error;
      await fetchRules();
    } catch (error) {
      console.error('Error deleting rule:', error);
    }
  };

  const handleToggleActive = async (rule: TransactionRule) => {
    try {
      const { error } = await supabase
        .from('transaction_rules')
        .update({ is_active: !rule.is_active })
        .eq('id', rule.id);

      if (error) throw error;
      await fetchRules();
    } catch (error) {
      console.error('Error toggling rule:', error);
    }
  };

  if (loading) {
    return <div className="p-4">Loading rules...</div>;
  }

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Transaction Rules</CardTitle>
              <CardDescription>Automatically categorize expenses based on rules</CardDescription>
            </div>
            <Button onClick={() => {
              setEditingRule(null);
              setShowForm(true);
            }}>
              <Plus className="mr-2 h-4 w-4" />
              Create Rule
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {rules.length === 0 ? (
            <div className="py-8 text-center text-muted-foreground">
              <p>No rules yet. Create your first rule to automatically categorize expenses.</p>
            </div>
          ) : (
            <div className="space-y-2">
              {rules.map((rule) => {
                const category = categories.find((c) => c.id === rule.category_id);
                return (
                  <div
                    key={rule.id}
                    className="flex items-center justify-between rounded-md border border-border p-3"
                  >
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <h4 className="font-medium">{rule.name}</h4>
                        <span className="text-xs text-muted-foreground">Priority: {rule.priority}</span>
                        {!rule.is_active && (
                          <span className="text-xs text-muted-foreground">(Inactive)</span>
                        )}
                      </div>
                      <p className="text-sm text-muted-foreground">
                        Match: {rule.match_type} → {category?.name || 'Uncategorized'}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => handleToggleActive(rule)}
                        className="p-1 hover:bg-hover rounded"
                        title={rule.is_active ? 'Deactivate' : 'Activate'}
                      >
                        {rule.is_active ? (
                          <ToggleRight className="h-5 w-5 text-accent" />
                        ) : (
                          <ToggleLeft className="h-5 w-5 text-muted-foreground" />
                        )}
                      </button>
                      <button
                        onClick={() => {
                          setEditingRule(rule);
                          setShowForm(true);
                        }}
                        className="p-1 hover:bg-hover rounded"
                      >
                        <Edit2 className="h-4 w-4" />
                      </button>
                      <button
                        onClick={() => handleDelete(rule.id)}
                        className="p-1 hover:bg-hover rounded text-destructive"
                      >
                        <Trash2 className="h-4 w-4" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {showForm && (
        <RuleForm
          rule={editingRule}
          categories={categories}
          onSubmit={editingRule ? handleUpdate : handleCreate}
          onCancel={() => {
            setShowForm(false);
            setEditingRule(null);
          }}
        />
      )}
    </>
  );
}

