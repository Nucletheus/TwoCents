export interface User {
  id: string;
  email: string;
  name: string | null;
  avatar_url: string | null;
  created_at: string;
}

export interface Household {
  id: string;
  name: string;
  created_at: string;
}

export interface HouseholdMember {
  user_id: string;
  household_id: string;
  role: 'owner' | 'member';
  joined_at: string;
}

export interface Category {
  id: string;
  name: string;
  icon: string | null;
  color: string | null;
  group_name: string | null;
  parent_color: string | null;
  household_id: string | null;
  is_default: boolean;
  is_savings_category: boolean;
  exclude_from_calculations: boolean;
  is_income_category: boolean;
  created_at: string;
}

export interface Account {
  id: string;
  user_id: string;
  household_id: string | null;
  name: string;
  type: 'chequing' | 'savings' | 'credit_card' | 'investment' | 'other';
  balance: number;
  created_at: string;
  updated_at: string;
}

export interface Expense {
  id: string;
  household_id: string;
  payer_id: string;
  account_id: string | null;
  amount: number;
  category_id: string;
  description: string | null;
  sub_description: string | null;
  vendor: string | null;
  status: 'pending_review' | 'approved' | 'flagged' | 'dismissed';
  reviewed_by: string | null;
  reviewed_at: string | null;
  date: string;
  receipt_url: string | null;
  created_at: string;
}

export interface ExpenseSplit {
  expense_id: string;
  user_id: string;
  amount: number;
  percentage: number | null;
}

export interface SavingsGoal {
  id: string;
  household_id: string;
  name: string;
  target_amount: number;
  deadline: string | null;
  account_id: string | null;
  priority: number;
  created_at: string;
}

export interface GoalContribution {
  id: string;
  goal_id: string;
  user_id: string;
  amount: number;
  date: string;
  expense_id: string | null;
}

export interface UnallocatedSavings {
  id: string;
  household_id: string;
  expense_id: string | null;
  amount: number;
  date: string;
  description: string | null;
  allocated_at: string | null;
  allocated_to_goal_id: string | null;
  created_at: string;
}

export interface Budget {
  id: string;
  household_id: string;
  budget_set_id: string;
  category_id: string;
  amount: number;
  period: 'monthly' | 'yearly';
  created_at: string;
}

export interface BudgetSet {
  id: string;
  household_id: string;
  name: string;
  is_main: boolean;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export type SplitType = 'equal' | 'percentage' | 'custom';

export interface TransactionRule {
  id: string;
  household_id: string;
  user_id: string;
  name: string;
  priority: number;
  match_type: 'vendor' | 'description' | 'amount_range' | 'date_pattern' | 'combination';
  match_pattern: RuleMatchPattern;
  category_id: string | null;
  is_active: boolean;
  created_at: string;
}

export interface RuleMatchPattern {
  vendor?: string | RegExp;
  description?: string | RegExp;
  sub_description?: string | RegExp;
  amountRange?: { min?: number; max?: number };
  datePattern?: string; // e.g., "first-of-month", "weekend", "monthly"
  combination?: 'AND' | 'OR';
}

export interface ExpenseFlag {
  id: string;
  expense_id: string;
  user_id: string;
  flag_type: 'review' | 'shared' | 'needs_attention';
  notes: string | null;
  resolved_at: string | null;
  created_at: string;
}

export interface VendorReviewFlag {
  id: string;
  household_id: string;
  vendor_name: string;
  requires_review: boolean;
  created_at: string;
}

export interface CSVImportPreset {
  id: string;
  user_id: string;
  household_id: string;
  preset_name: string;
  filename_pattern: string | null;
  column_mapping: ColumnMapping;
  default_payer_id: string | null;
  default_category_id: string | null;
  skip_columns: string[];
  last_used_at: string | null;
  created_at: string;
}

export interface ColumnMapping {
  date?: string;
  amount?: string | string[];
  description?: string;
  vendor?: string;
  category?: string;
  [key: string]: string | string[] | undefined;
}

export interface ExpenseComment {
  id: string;
  expense_id: string;
  user_id: string;
  comment: string;
  created_at: string;
}

