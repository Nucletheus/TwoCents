'use client';

import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog';
import Input from '../components/ui/Input';
import Button from '../components/ui/Button';

interface FlagCommentDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  transactionCount: number;
  onConfirm: (comment: string) => void;
  onCancel: () => void;
}

export default function FlagCommentDialog({
  open,
  onOpenChange,
  transactionCount,
  onConfirm,
  onCancel,
}: FlagCommentDialogProps) {
  const [comment, setComment] = useState('');

  const handleSubmit = () => {
    onConfirm(comment.trim() || 'Flagged for review');
    setComment('');
    onOpenChange(false);
  };

  const handleCancel = () => {
    setComment('');
    onCancel();
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Flag Transaction{transactionCount > 1 ? 's' : ''}</DialogTitle>
          <DialogDescription>
            {transactionCount === 1
              ? 'Add a comment to explain why this transaction needs review.'
              : `Add a comment for ${transactionCount} transactions. This comment will apply to all selected transactions.`}
          </DialogDescription>
        </DialogHeader>
        <div className="py-4">
          <textarea
            value={comment}
            onChange={(e) => setComment(e.target.value)}
            placeholder="Why does this transaction need review? (optional)"
            className="flex min-h-[100px] w-full rounded-notion border border-border bg-background px-3 py-2 text-sm resize-none placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-0 disabled:cursor-not-allowed disabled:opacity-50"
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                // Enter (without Shift) submits
                e.preventDefault();
                handleSubmit();
              }
              // Shift+Enter allows newlines (default behavior)
            }}
          />
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={handleCancel}>
            Cancel
          </Button>
          <Button onClick={handleSubmit}>
            Flag Transaction{transactionCount > 1 ? 's' : ''}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

