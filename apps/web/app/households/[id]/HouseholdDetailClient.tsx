'use client';

import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
import Link from 'next/link';
import MainLayout from '../../components/MainLayout';
import Button from '../../components/ui/Button';
import { ArrowLeft, Users, Wallet, Tag } from 'lucide-react';
import PartnersTab from './tabs/PartnersTab';
import AccountsTab from './tabs/AccountsTab';
import CategoriesTab from './tabs/CategoriesTab';

interface HouseholdMember {
  user_id: string;
  role: string;
  profiles: {
    name: string | null;
  };
}

type TabId = 'partners' | 'accounts' | 'categories';

const TABS: { id: TabId; label: string; icon: React.ReactNode }[] = [
  { id: 'partners', label: 'Partners', icon: <Users className="h-4 w-4" /> },
  { id: 'accounts', label: 'Accounts', icon: <Wallet className="h-4 w-4" /> },
  { id: 'categories', label: 'Categories', icon: <Tag className="h-4 w-4" /> },
];

export default function HouseholdDetailClient({ householdId }: { householdId: string }) {
  const supabase = createClient();
  const [household, setHousehold] = useState<any>(null);
  const [members, setMembers] = useState<HouseholdMember[]>([]);
  const [currentUserId, setCurrentUserId] = useState<string>('');
  const [currentUserRole, setCurrentUserRole] = useState<string>('member');
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<TabId>('partners');

  useEffect(() => {
    fetchHouseholdData();
  }, [householdId]);

  const fetchHouseholdData = async () => {
    try {
      // Get current user
      const {
        data: { user },
      } = await supabase.auth.getUser();

      if (user) {
        setCurrentUserId(user.id);
      }

      // Fetch household
      const { data: householdData, error: householdError } = await supabase
        .from('households')
        .select('*')
        .eq('id', householdId)
        .single();

      if (householdError) throw householdError;

      // Fetch members
      const { data: membersData, error: membersError } = await supabase
        .from('household_members')
        .select('user_id, role')
        .eq('household_id', householdId);

      if (membersError) throw membersError;

      // Fetch profiles for members
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

      // Combine members with profiles
      const enrichedMembers = (membersData || []).map((m: any) => ({
        user_id: m.user_id,
        role: m.role,
        profiles: profilesMap[m.user_id] || { name: null },
      }));

      setHousehold(householdData);
      setMembers(enrichedMembers);

      // Set current user's role
      if (user) {
        const currentMember = enrichedMembers.find((m: any) => m.user_id === user.id);
        if (currentMember) {
          setCurrentUserRole(currentMember.role);
        }
      }
    } catch (error) {
      console.error('Error fetching household data:', error);
    } finally {
      setLoading(false);
    }
  };

  // Convert members to the format needed by AccountsTab
  const membersForAccounts = members.map((m) => ({
    user_id: m.user_id,
    profiles: m.profiles ? { name: m.profiles.name } : null,
  }));

  return (
    <MainLayout
      pageHeader={{
        title: household?.name || 'Household',
        subtitle: 'Manage household settings',
        showHouseholdSelect: false,
      }}
    >
      {loading ? (
        <div className="flex items-center justify-center py-24">
          <div className="text-muted-foreground">Loading...</div>
        </div>
      ) : !household ? (
        <div className="flex items-center justify-center py-24">
          <div className="text-muted-foreground">Household not found</div>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Back Button */}
          <Link href="/households">
            <Button variant="ghost" size="sm" className="gap-2">
              <ArrowLeft className="h-4 w-4" />
              Back to Households
            </Button>
          </Link>

          {/* Tab Navigation */}
          <div className="border-b border-border">
            <nav className="flex gap-1" aria-label="Tabs">
              {TABS.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`
                    flex items-center gap-2 px-4 py-3 text-sm font-medium transition-colors
                    border-b-2 -mb-px
                    ${
                      activeTab === tab.id
                        ? 'border-accent text-accent'
                        : 'border-transparent text-muted-foreground hover:text-foreground hover:border-border'
                    }
                  `}
                >
                  {tab.icon}
                  {tab.label}
                </button>
              ))}
            </nav>
          </div>

          {/* Tab Content */}
          <div className="min-h-[400px]">
            {activeTab === 'partners' && (
              <PartnersTab
                householdId={householdId}
                members={members}
                currentUserId={currentUserId}
                currentUserRole={currentUserRole}
                supabase={supabase}
                onMembersChange={fetchHouseholdData}
              />
            )}

            {activeTab === 'accounts' && (
              <AccountsTab
                householdId={householdId}
                members={membersForAccounts}
                supabase={supabase}
              />
            )}

            {activeTab === 'categories' && (
              <CategoriesTab
                householdId={householdId}
                supabase={supabase}
              />
            )}
          </div>
        </div>
      )}
    </MainLayout>
  );
}
