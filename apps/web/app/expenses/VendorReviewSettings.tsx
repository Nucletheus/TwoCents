'use client';

import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
import type { VendorReviewFlag } from '@twocents/shared';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import Input from '../components/ui/Input';
import Button from '../components/ui/Button';
import { Plus, Trash2, ToggleLeft, ToggleRight } from 'lucide-react';

interface VendorReviewSettingsProps {
  householdId: string | null;
}

export default function VendorReviewSettings({ householdId }: VendorReviewSettingsProps) {
  const supabase = createClient();
  const [vendors, setVendors] = useState<VendorReviewFlag[]>([]);
  const [newVendor, setNewVendor] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (householdId) {
      fetchVendors();
    }
  }, [householdId]);

  const fetchVendors = async () => {
    if (!householdId) return;

    try {
      const { data, error } = await supabase
        .from('vendor_review_flags')
        .select('*')
        .eq('household_id', householdId)
        .order('vendor_name', { ascending: true });

      if (error) throw error;
      setVendors(data || []);
    } catch (error) {
      console.error('Error fetching vendor flags:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleAddVendor = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newVendor.trim() || !householdId) return;

    try {
      const { error } = await supabase.from('vendor_review_flags').insert({
        household_id: householdId,
        vendor_name: newVendor.trim(),
        requires_review: true,
      });

      if (error) throw error;
      setNewVendor('');
      await fetchVendors();
    } catch (error) {
      console.error('Error adding vendor:', error);
    }
  };

  const handleToggleReview = async (vendor: VendorReviewFlag) => {
    try {
      const { error } = await supabase
        .from('vendor_review_flags')
        .update({ requires_review: !vendor.requires_review })
        .eq('id', vendor.id);

      if (error) throw error;
      await fetchVendors();
    } catch (error) {
      console.error('Error toggling vendor review:', error);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Are you sure you want to remove this vendor?')) return;

    try {
      const { error } = await supabase.from('vendor_review_flags').delete().eq('id', id);
      if (error) throw error;
      await fetchVendors();
    } catch (error) {
      console.error('Error deleting vendor:', error);
    }
  };

  if (loading) {
    return <div className="p-4">Loading vendor settings...</div>;
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Vendor Review Settings</CardTitle>
        <CardDescription>Mark vendors that require review before approval</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <form onSubmit={handleAddVendor} className="flex gap-2">
          <Input
            value={newVendor}
            onChange={(e) => setNewVendor(e.target.value)}
            placeholder="Vendor name (e.g., AMAZON)"
            className="flex-1"
          />
          <Button type="submit">
            <Plus className="mr-2 h-4 w-4" />
            Add
          </Button>
        </form>

        {vendors.length === 0 ? (
          <div className="py-4 text-center text-muted-foreground">
            <p>No vendors marked for review yet.</p>
          </div>
        ) : (
          <div className="space-y-2">
            {vendors.map((vendor) => (
              <div
                key={vendor.id}
                className="flex items-center justify-between rounded-md border border-border p-3"
              >
                <span className="font-medium">{vendor.vendor_name}</span>
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => handleToggleReview(vendor)}
                    className="p-1 hover:bg-hover rounded"
                    title={vendor.requires_review ? 'Disable review' : 'Enable review'}
                  >
                    {vendor.requires_review ? (
                      <ToggleRight className="h-5 w-5 text-accent" />
                    ) : (
                      <ToggleLeft className="h-5 w-5 text-muted-foreground" />
                    )}
                  </button>
                  <button
                    onClick={() => handleDelete(vendor.id)}
                    className="p-1 hover:bg-hover rounded text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

