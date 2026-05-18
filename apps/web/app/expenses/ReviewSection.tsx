'use client';

import type { Category } from '@twocents/shared';
import ReviewGrid from './ReviewGrid';

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

interface ReviewSectionProps {
  householdId: string | null;
  categories: Category[];
  householdMembers?: HouseholdMember[];
  accounts?: Account[];
  onExpenseUpdated: () => void;
  transactionsFlaggedCounter?: number;
}

export default function ReviewSection({ 
  householdId, 
  categories, 
  householdMembers = [],
  accounts = [],
  onExpenseUpdated,
  transactionsFlaggedCounter
}: ReviewSectionProps) {
  return (
    <ReviewGrid
      householdId={householdId}
      categories={categories}
      householdMembers={householdMembers}
      accounts={accounts}
      onExpenseUpdated={onExpenseUpdated}
      transactionsFlaggedCounter={transactionsFlaggedCounter}
    />
  );
}

