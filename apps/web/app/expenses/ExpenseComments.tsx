'use client';

import { useState, useEffect, useMemo } from 'react';
import { createClient } from '@/lib/supabase/client';
import type { ExpenseComment } from '@twocents/shared';
import { formatDate } from '@twocents/shared';
import Input from '../components/ui/Input';
import Button from '../components/ui/Button';
import { Send, Trash2, Check, X } from 'lucide-react';

interface HouseholdMember {
  user_id: string;
  profiles: {
    name: string | null;
    email: string;
  } | null;
}

interface ExpenseCommentsProps {
  expenseId: string;
  householdId?: string | null;
  userColors?: Record<string, string>;
  currentUserId?: string | null;
  expenseStatus?: string | null;
  reviewedBy?: string | null;
  reviewedAt?: string | null;
  householdMembers?: HouseholdMember[];
  onApprove?: (expenseId: string) => Promise<void>;
  onDismiss?: (expenseId: string) => Promise<void>;
  onStatusUpdate?: () => Promise<void>;
}

export default function ExpenseComments({ 
  expenseId, 
  householdId, 
  userColors: propUserColors,
  currentUserId: propCurrentUserId,
  expenseStatus,
  reviewedBy,
  reviewedAt,
  householdMembers = [],
  onApprove,
  onDismiss,
  onStatusUpdate
}: ExpenseCommentsProps) {
  const supabase = createClient();
  const [comments, setComments] = useState<ExpenseComment[]>([]);
  const [newComment, setNewComment] = useState('');
  const [loading, setLoading] = useState(true);
  const [currentUserId, setCurrentUserId] = useState<string | null>(propCurrentUserId || null);
  const [userColors, setUserColors] = useState<Record<string, string>>(propUserColors || {});
  const [localStatus, setLocalStatus] = useState(expenseStatus);
  const [localReviewedBy, setLocalReviewedBy] = useState(reviewedBy);
  const [localReviewedAt, setLocalReviewedAt] = useState(reviewedAt);

  useEffect(() => {
    fetchComments();
    if (!propCurrentUserId) {
      fetchCurrentUser();
    }
    if (!propUserColors && householdId) {
      fetchUserColors();
    }
  }, [expenseId, householdId, propCurrentUserId, propUserColors]);

  // Update local status when props change
  useEffect(() => {
    setLocalStatus(expenseStatus);
    setLocalReviewedBy(reviewedBy);
    setLocalReviewedAt(reviewedAt);
  }, [expenseStatus, reviewedBy, reviewedAt]);

  const fetchCurrentUser = async () => {
    const {
      data: { user },
    } = await supabase.auth.getUser();
    setCurrentUserId(user?.id || null);
  };

  const fetchUserColors = async () => {
    if (!householdId) return;
    
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
  };

  const fetchComments = async () => {
    setLoading(true);
    try {
      // Fetch comments without profiles join (PostgREST can't join through auth.users)
      const { data: commentsData, error: commentsError } = await supabase
        .from('expense_comments')
        .select('*')
        .eq('expense_id', expenseId)
        .order('created_at', { ascending: true });

      if (commentsError) {
        throw commentsError;
      }

      if (!commentsData || commentsData.length === 0) {
        setComments([]);
        setLoading(false);
        return;
      }

      // Extract unique user IDs
      const userIds = [...new Set(commentsData.map(c => c.user_id))];
      
      // Fetch profiles for those user IDs
      const { data: profilesData } = await supabase
        .from('profiles')
        .select('id, name')
        .in('id', userIds);

      // Create a map of user_id -> profile
      const profilesMap = new Map(
        (profilesData || []).map(p => [p.id, p])
      );

      // Join comments with profiles
      const commentsWithProfiles = commentsData.map(comment => ({
        ...comment,
        profiles: profilesMap.get(comment.user_id) || null,
      }));
      
      setComments(commentsWithProfiles);
    } catch (error) {
      console.error('Error fetching comments:', error);
      setComments([]);
    } finally {
      setLoading(false);
    }
  };

  const handleAddComment = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newComment.trim() || !currentUserId) return;

    const commentText = newComment.trim();
    setNewComment(''); // Clear input immediately for better UX

    // Optimistic update: add comment to local state immediately
    const currentUserProfile = householdMembers.find(m => m.user_id === currentUserId)?.profiles || null;
    const optimisticComment = {
      id: `temp-${Date.now()}`,
      expense_id: expenseId,
      user_id: currentUserId,
      comment: commentText,
      created_at: new Date().toISOString(),
      profiles: currentUserProfile,
    };
    
    setComments(prev => [...prev, optimisticComment]);

    try {
      const { data, error } = await supabase.from('expense_comments').insert({
        expense_id: expenseId,
        user_id: currentUserId,
        comment: commentText,
      }).select().single();

      if (error) throw error;

      // Replace optimistic comment with real one
      setComments(prev => prev.map(c => 
        c.id === optimisticComment.id ? { ...data, profiles: currentUserProfile } : c
      ));
    } catch (error) {
      console.error('Error adding comment:', error);
      // Remove optimistic comment on error
      setComments(prev => prev.filter(c => c.id !== optimisticComment.id));
      setNewComment(commentText); // Restore the comment text
    }
  };

  const handleDeleteComment = async (commentId: string) => {
    if (!confirm('Are you sure you want to delete this comment?')) return;

    try {
      const { error } = await supabase
        .from('expense_comments')
        .delete()
        .eq('id', commentId);

      if (error) throw error;
      await fetchComments();
    } catch (error) {
      console.error('Error deleting comment:', error);
    }
  };

  const handleApprove = async () => {
    if (!onApprove) return;
    try {
      await onApprove(expenseId);
      // Fetch updated expense data directly instead of calling onStatusUpdate
      const { data: expenseData } = await supabase
        .from('expenses')
        .select('status, reviewed_by, reviewed_at')
        .eq('id', expenseId)
        .single();
      
      if (expenseData) {
        setLocalStatus(expenseData.status);
        setLocalReviewedBy(expenseData.reviewed_by);
        setLocalReviewedAt(expenseData.reviewed_at);
      }
    } catch (error) {
      console.error('Error approving:', error);
    }
  };

  const handleDismiss = async () => {
    if (!onDismiss) return;
    try {
      await onDismiss(expenseId);
      // Fetch updated expense data directly instead of calling onStatusUpdate
      const { data: expenseData } = await supabase
        .from('expenses')
        .select('status, reviewed_by, reviewed_at')
        .eq('id', expenseId)
        .single();
      
      if (expenseData) {
        setLocalStatus(expenseData.status);
        setLocalReviewedBy(expenseData.reviewed_by);
        setLocalReviewedAt(expenseData.reviewed_at);
      }
    } catch (error) {
      console.error('Error dismissing:', error);
    }
  };

  // Check if transaction can be approved/dismissed (not already reviewed)
  const canApproveOrDismiss = localStatus === 'pending_review' || localStatus === 'flagged';

  // Helper function to determine if a color is light or dark
  const isLightColor = (color: string): boolean => {
    // Remove # if present
    const hex = color.replace('#', '');
    const r = parseInt(hex.substring(0, 2), 16);
    const g = parseInt(hex.substring(2, 4), 16);
    const b = parseInt(hex.substring(4, 6), 16);
    // Calculate luminance
    const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
    return luminance > 0.5;
  };

  // Get user color with fallback
  const getUserColor = (userId: string): string => {
    return userColors[userId] || '#3b82f6'; // Default to accent color
  };

  // Get text color based on background
  const getTextColor = (bgColor: string): string => {
    return isLightColor(bgColor) ? '#000000' : '#ffffff';
  };

  // Create a unified timeline of messages (comments + approve/dismiss actions)
  const getTimelineMessages = useMemo(() => {
    const messages: Array<{
      type: 'comment' | 'approved' | 'dismissed';
      id: string;
      timestamp: string;
      data: any;
    }> = [];

    // Add comments
    comments.forEach(comment => {
      messages.push({
        type: 'comment',
        id: comment.id,
        timestamp: comment.created_at,
        data: comment,
      });
    });

    // Add approve/dismiss action if it exists
    if (localStatus && (localStatus === 'approved' || localStatus === 'dismissed') && localReviewedAt) {
      messages.push({
        type: localStatus === 'approved' ? 'approved' : 'dismissed',
        id: `action-${localReviewedAt}`,
        timestamp: localReviewedAt,
        data: {
          reviewed_by: localReviewedBy,
          reviewed_at: localReviewedAt,
        },
      });
    }

    // Sort by timestamp
    return messages.sort((a, b) => 
      new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );
  }, [comments, localStatus, localReviewedBy, localReviewedAt]);

  if (loading) {
    return <div className="text-sm text-muted-foreground">Loading comments...</div>;
  }

  return (
    <div className="flex flex-col h-full min-h-[400px] max-h-[600px]">
      {/* Messages area */}
      <div className="flex-1 overflow-y-auto space-y-3 p-4">
        {getTimelineMessages.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <p className="text-sm text-muted-foreground">No comments yet.</p>
          </div>
        ) : (
          getTimelineMessages.map((message) => {
            if (message.type === 'comment') {
              // Render comment
              const comment = message.data;
              const isOwnMessage = comment.user_id === currentUserId;
              const userColor = getUserColor(comment.user_id);
              const textColor = getTextColor(userColor);
              const senderName = comment.profiles?.name || comment.profiles?.email || 'Unknown';
              const timestamp = formatDate(comment.created_at, 'HH:mm');

              return (
                <div
                  key={comment.id}
                  className={`flex items-end gap-2 ${isOwnMessage ? 'flex-row-reverse' : 'flex-row'}`}
                >
                  {/* Avatar/Initial for other users */}
                  {!isOwnMessage && (
                    <div
                      className="flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-xs font-medium text-white shadow-sm"
                      style={{ backgroundColor: userColor }}
                    >
                      {senderName.charAt(0).toUpperCase()}
                    </div>
                  )}

                  <div className={`flex flex-col ${isOwnMessage ? 'items-end' : 'items-start'} max-w-[75%]`}>
                    {/* Sender name for other users */}
                    {!isOwnMessage && (
                      <span className="text-xs font-medium text-muted-foreground mb-1 px-1">
                        {senderName}
                      </span>
                    )}

                    {/* Message bubble */}
                    <div
                      className="rounded-2xl px-4 py-2 shadow-sm relative group"
                      style={{
                        backgroundColor: userColor,
                        color: textColor,
                      }}
                    >
                      <p className="text-sm break-words">{comment.comment}</p>
                      
                      {/* Delete button for own messages */}
                      {isOwnMessage && (
                        <button
                          onClick={() => handleDeleteComment(comment.id)}
                          className="absolute -top-1 -right-1 opacity-0 group-hover:opacity-100 transition-opacity bg-red-500 text-white rounded-full p-1 hover:bg-red-600 shadow-md"
                          title="Delete comment"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      )}
                    </div>

                    {/* Timestamp */}
                    <span className="text-xs text-muted-foreground mt-1 px-1">
                      {timestamp}
                    </span>
                  </div>
                </div>
              );
            } else {
              // Render approve/dismiss action - more compact
              const isApproved = message.type === 'approved';
              const reviewer = householdMembers.find(m => m.user_id === message.data.reviewed_by);
              const reviewerName = reviewer?.profiles?.name || reviewer?.profiles?.email || 'Unknown';
              const timestamp = formatDate(message.data.reviewed_at, 'HH:mm');

              return (
                <div key={message.id} className="flex items-center justify-center">
                  <div className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium ${
                    isApproved
                      ? 'bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300'
                      : 'bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300'
                  }`}>
                    <span>{isApproved ? '✓' : '✗'}</span>
                    <span>{isApproved ? 'Approved' : 'Dismissed'}</span>
                    <span className="text-muted-foreground">by {reviewerName}</span>
                    <span className="text-muted-foreground">•</span>
                    <span className="text-muted-foreground">{timestamp}</span>
                  </div>
                </div>
              );
            }
          })
        )}
      </div>

      {/* Input area */}
      <div className="border-t border-border p-4 bg-background">
        {/* Approve/Dismiss buttons - only show if not already reviewed */}
        {canApproveOrDismiss && (onApprove || onDismiss) && (
          <div className="flex gap-2 mb-3">
            {onApprove && (
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={handleApprove}
                className="flex-1 text-green-600 border-green-300 hover:bg-green-50 dark:hover:bg-green-900/20"
              >
                <Check className="h-4 w-4 mr-1" />
                Approve
              </Button>
            )}
            {onDismiss && (
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={handleDismiss}
                className="flex-1 text-orange-600 border-orange-300 hover:bg-orange-50 dark:hover:bg-orange-900/20"
              >
                <X className="h-4 w-4 mr-1" />
                Dismiss
              </Button>
            )}
          </div>
        )}
        
        <form onSubmit={handleAddComment} className="flex gap-2">
          <Input
            value={newComment}
            onChange={(e) => setNewComment(e.target.value)}
            placeholder="Type a message..."
            className="flex-1"
          />
          <Button type="submit" size="sm" disabled={!newComment.trim()}>
            <Send className="h-4 w-4" />
          </Button>
        </form>
      </div>
    </div>
  );
}

