'use client';

import { useState, useEffect, useCallback } from 'react';
import { SupabaseClient } from '@supabase/supabase-js';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../../components/ui/Card';
import Input from '../../../components/ui/Input';
import Button from '../../../components/ui/Button';
import Select from '../../../components/ui/Select';
import { UserPlus, Trash2, Crown, User, Mail, Clock, X, Check } from 'lucide-react';

interface HouseholdMember {
  user_id: string;
  role: string;
  profiles: {
    name: string | null;
  };
}

interface Invitation {
  id: string;
  email: string;
  role: string;
  status: string;
  expires_at: string;
  created_at: string;
}

interface PartnersTabProps {
  householdId: string;
  members: HouseholdMember[];
  currentUserId: string;
  currentUserRole: string;
  supabase: SupabaseClient;
  onMembersChange: () => void;
}

export default function PartnersTab({
  householdId,
  members,
  currentUserId,
  currentUserRole,
  supabase,
  onMembersChange,
}: PartnersTabProps) {
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteRole, setInviteRole] = useState<'member' | 'owner'>('member');
  const [isInviting, setIsInviting] = useState(false);
  const [inviteError, setInviteError] = useState<string | null>(null);
  const [inviteSuccess, setInviteSuccess] = useState<string | null>(null);
  const [removingId, setRemovingId] = useState<string | null>(null);
  const [updatingRoleId, setUpdatingRoleId] = useState<string | null>(null);
  const [confirmRemoveId, setConfirmRemoveId] = useState<string | null>(null);
  
  // Invitations state
  const [invitations, setInvitations] = useState<Invitation[]>([]);
  const [loadingInvitations, setLoadingInvitations] = useState(true);
  const [cancellingInviteId, setCancellingInviteId] = useState<string | null>(null);
  
  // User colors state
  const [userColors, setUserColors] = useState<Record<string, string>>({});
  const [updatingColorId, setUpdatingColorId] = useState<string | null>(null);

  const isOwner = currentUserRole === 'owner';

  const fetchInvitations = useCallback(async () => {
    setLoadingInvitations(true);
    try {
      const { data, error } = await supabase
        .from('household_invitations')
        .select('*')
        .eq('household_id', householdId)
        .eq('status', 'pending')
        .order('created_at', { ascending: false });

      if (error) throw error;
      setInvitations(data || []);
    } catch (error) {
      console.error('Error fetching invitations:', error);
    } finally {
      setLoadingInvitations(false);
    }
  }, [supabase, householdId]);

  useEffect(() => {
    fetchInvitations();
    fetchUserColors();
  }, [fetchInvitations]);

  const fetchUserColors = useCallback(async () => {
    try {
      const { data, error } = await supabase
        .from('household_member_colors')
        .select('user_id, color')
        .eq('household_id', householdId);

      if (error) throw error;

      const colorsMap: Record<string, string> = {};
      (data || []).forEach((item: any) => {
        colorsMap[item.user_id] = item.color;
      });
      setUserColors(colorsMap);
    } catch (error) {
      console.error('Error fetching user colors:', error);
    }
  }, [supabase, householdId]);

  const handleColorChange = useCallback(async (userId: string, color: string) => {
    // Only allow users to set their own color
    if (userId !== currentUserId) return;

    setUpdatingColorId(userId);
    try {
      const { error } = await supabase
        .from('household_member_colors')
        .upsert({
          household_id: householdId,
          user_id: userId,
          color: color,
          updated_at: new Date().toISOString(),
        }, {
          onConflict: 'household_id,user_id'
        });

      if (error) throw error;

      setUserColors((prev) => ({
        ...prev,
        [userId]: color,
      }));
    } catch (error) {
      console.error('Error updating user color:', error);
    } finally {
      setUpdatingColorId(null);
    }
  }, [supabase, householdId, currentUserId]);

  const handleInvite = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inviteEmail.trim()) return;

    setIsInviting(true);
    setInviteError(null);
    setInviteSuccess(null);

    try {
      // Get the session for the auth token
      const { data: { session } } = await supabase.auth.getSession();
      if (!session) {
        setInviteError('You must be logged in to send invitations');
        return;
      }

      // Call the edge function
      const response = await fetch(
        `${process.env.NEXT_PUBLIC_SUPABASE_URL}/functions/v1/send-household-invitation`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${session.access_token}`,
          },
          body: JSON.stringify({
            householdId,
            email: inviteEmail.trim().toLowerCase(),
            role: inviteRole,
          }),
        }
      );

      const result = await response.json();

      if (!response.ok) {
        setInviteError(result.error || 'Failed to send invitation');
        return;
      }

      setInviteSuccess(result.message);
      setInviteEmail('');
      setInviteRole('member');
      fetchInvitations();
    } catch (error) {
      console.error('Error inviting member:', error);
      setInviteError('Failed to send invitation. Please try again.');
    } finally {
      setIsInviting(false);
    }
  };

  const handleCancelInvitation = async (invitationId: string) => {
    setCancellingInviteId(invitationId);
    try {
      const { error } = await supabase
        .from('household_invitations')
        .update({ status: 'cancelled' })
        .eq('id', invitationId);

      if (error) throw error;
      fetchInvitations();
    } catch (error) {
      console.error('Error cancelling invitation:', error);
    } finally {
      setCancellingInviteId(null);
    }
  };

  const handleResendInvitation = async (invitation: Invitation) => {
    setCancellingInviteId(invitation.id);
    try {
      // Cancel the old one and create a new one
      await supabase
        .from('household_invitations')
        .update({ status: 'cancelled' })
        .eq('id', invitation.id);

      // Get the session for the auth token
      const { data: { session } } = await supabase.auth.getSession();
      if (!session) return;

      // Call the edge function to create a new invitation
      await fetch(
        `${process.env.NEXT_PUBLIC_SUPABASE_URL}/functions/v1/send-household-invitation`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${session.access_token}`,
          },
          body: JSON.stringify({
            householdId,
            email: invitation.email,
            role: invitation.role,
          }),
        }
      );

      fetchInvitations();
    } catch (error) {
      console.error('Error resending invitation:', error);
    } finally {
      setCancellingInviteId(null);
    }
  };

  const handleUpdateRole = async (userId: string, newRole: string) => {
    if (!isOwner || userId === currentUserId) return;

    setUpdatingRoleId(userId);
    try {
      const { error } = await supabase
        .from('household_members')
        .update({ role: newRole })
        .eq('household_id', householdId)
        .eq('user_id', userId);

      if (error) throw error;
      onMembersChange();
    } catch (error) {
      console.error('Error updating role:', error);
      alert('Failed to update member role');
    } finally {
      setUpdatingRoleId(null);
    }
  };

  const handleRemoveMember = async () => {
    if (!confirmRemoveId || !isOwner) return;

    setRemovingId(confirmRemoveId);
    try {
      const { error } = await supabase
        .from('household_members')
        .delete()
        .eq('household_id', householdId)
        .eq('user_id', confirmRemoveId);

      if (error) throw error;
      setConfirmRemoveId(null);
      onMembersChange();
    } catch (error) {
      console.error('Error removing member:', error);
      alert('Failed to remove member');
    } finally {
      setRemovingId(null);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  };

  const isExpired = (expiresAt: string) => {
    return new Date(expiresAt) < new Date();
  };

  return (
    <div className="space-y-6">
      {/* Current Members */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <User className="h-5 w-5" />
            Household Members
          </CardTitle>
          <CardDescription>
            {members.length} {members.length === 1 ? 'member' : 'members'} in this household
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {members.map((member) => {
            const isCurrentUser = member.user_id === currentUserId;
            const canManage = isOwner && !isCurrentUser;

            return (
              <div
                key={member.user_id}
                className="flex items-center justify-between gap-4 rounded-notion border border-border p-3 hover:bg-hover/50 transition-colors"
              >
                <div className="flex items-center gap-3 min-w-0 flex-1">
                  <div className="flex h-9 w-9 items-center justify-center rounded-full bg-accent/10 text-accent shrink-0">
                    {member.role === 'owner' ? (
                      <Crown className="h-4 w-4" />
                    ) : (
                      <User className="h-4 w-4" />
                    )}
                  </div>
                  <div className="min-w-0">
                    <p className="font-medium truncate text-sm">
                      {member.profiles?.name || 'Unknown User'}
                      {isCurrentUser && (
                        <span className="ml-2 text-xs text-muted-foreground">(you)</span>
                      )}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  {/* Color picker - only show for current user */}
                  {isCurrentUser && (
                    <input
                      type="color"
                      value={userColors[member.user_id] || '#3b82f6'}
                      onChange={(e) => handleColorChange(member.user_id, e.target.value)}
                      disabled={updatingColorId === member.user_id}
                      className="h-8 w-8 cursor-pointer rounded border border-border bg-transparent p-0 disabled:opacity-50"
                      title="Set your color for expense rows"
                    />
                  )}
                  {canManage ? (
                    <>
                      <Select
                        value={member.role}
                        onChange={(e) => handleUpdateRole(member.user_id, e.target.value)}
                        disabled={updatingRoleId === member.user_id}
                        className="h-8 w-24 text-sm"
                      >
                        <option value="owner">Owner</option>
                        <option value="member">Member</option>
                      </Select>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setConfirmRemoveId(member.user_id)}
                        disabled={removingId === member.user_id}
                        className="text-muted-foreground hover:text-red-600 h-8 w-8 p-0"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </>
                  ) : (
                    <span className="rounded-full bg-muted px-2.5 py-1 text-xs font-medium text-foreground capitalize">
                      {member.role}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </CardContent>
      </Card>

      {/* Pending Invitations */}
      {isOwner && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Mail className="h-5 w-5" />
              Pending Invitations
            </CardTitle>
            <CardDescription>
              {loadingInvitations 
                ? 'Loading...' 
                : `${invitations.length} pending ${invitations.length === 1 ? 'invitation' : 'invitations'}`
              }
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {loadingInvitations ? (
              <div className="py-4 text-center text-sm text-muted-foreground">Loading invitations...</div>
            ) : invitations.length === 0 ? (
              <div className="py-4 text-center text-sm text-muted-foreground">No pending invitations</div>
            ) : (
              invitations.map((invitation) => {
                const expired = isExpired(invitation.expires_at);
                return (
                  <div
                    key={invitation.id}
                    className={`flex items-center justify-between gap-4 rounded-notion border p-3 ${
                      expired ? 'border-red-200 bg-red-50/50' : 'border-border'
                    }`}
                  >
                    <div className="flex items-center gap-3 min-w-0 flex-1">
                      <div className={`flex h-9 w-9 items-center justify-center rounded-full shrink-0 ${
                        expired ? 'bg-red-100 text-red-600' : 'bg-yellow-100 text-yellow-600'
                      }`}>
                        {expired ? <X className="h-4 w-4" /> : <Clock className="h-4 w-4" />}
                      </div>
                      <div className="min-w-0">
                        <p className="font-medium truncate text-sm">{invitation.email}</p>
                        <p className="text-xs text-muted-foreground">
                          {expired 
                            ? 'Expired' 
                            : `Expires ${formatDate(invitation.expires_at)}`
                          }
                          {' · '}
                          <span className="capitalize">{invitation.role}</span>
                        </p>
                      </div>
                    </div>

                    <div className="flex items-center gap-1 shrink-0">
                      {expired ? (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleResendInvitation(invitation)}
                          disabled={cancellingInviteId === invitation.id}
                          className="text-xs h-8"
                        >
                          Resend
                        </Button>
                      ) : (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleResendInvitation(invitation)}
                          disabled={cancellingInviteId === invitation.id}
                          className="text-xs h-8"
                        >
                          Resend
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleCancelInvitation(invitation.id)}
                        disabled={cancellingInviteId === invitation.id}
                        className="text-muted-foreground hover:text-red-600 h-8 w-8 p-0"
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                );
              })
            )}
          </CardContent>
        </Card>
      )}

      {/* Invite Form */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <UserPlus className="h-5 w-5" />
            Invite Partner
          </CardTitle>
          <CardDescription>Add a new member to this household by email</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleInvite} className="space-y-3">
            <div className="flex gap-2">
              <Input
                type="email"
                value={inviteEmail}
                onChange={(e) => {
                  setInviteEmail(e.target.value);
                  setInviteError(null);
                  setInviteSuccess(null);
                }}
                placeholder="Enter email address"
                disabled={isInviting}
                className="flex-1"
              />
              <Select
                value={inviteRole}
                onChange={(e) => setInviteRole(e.target.value as 'member' | 'owner')}
                disabled={isInviting}
                className="w-28"
              >
                <option value="member">Member</option>
                <option value="owner">Owner</option>
              </Select>
              <Button type="submit" disabled={isInviting || !inviteEmail.trim()}>
                {isInviting ? 'Sending...' : 'Send Invite'}
              </Button>
            </div>
            
            {inviteError && (
              <p className="text-sm text-red-600 flex items-center gap-1">
                <X className="h-4 w-4" />
                {inviteError}
              </p>
            )}
            
            {inviteSuccess && (
              <p className="text-sm text-green-600 flex items-center gap-1">
                <Check className="h-4 w-4" />
                {inviteSuccess}
              </p>
            )}
            
            <p className="text-xs text-muted-foreground">
              The invited user will receive an email to join this household. If they don't have an account, they'll be added automatically when they sign up with this email.
            </p>
          </form>
        </CardContent>
      </Card>

      {/* Remove Member Confirmation Dialog */}
      {confirmRemoveId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <Card className="w-full max-w-md">
            <CardHeader>
              <CardTitle>Remove Member</CardTitle>
              <CardDescription>
                Are you sure you want to remove this member from the household? They will lose access to all household data.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex gap-2 justify-end">
                <Button
                  variant="ghost"
                  onClick={() => setConfirmRemoveId(null)}
                  disabled={removingId !== null}
                >
                  Cancel
                </Button>
                <Button
                  variant="danger"
                  onClick={handleRemoveMember}
                  disabled={removingId !== null}
                >
                  {removingId ? 'Removing...' : 'Remove'}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
