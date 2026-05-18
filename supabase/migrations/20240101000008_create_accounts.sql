-- Create accounts table
CREATE TABLE public.accounts (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  household_id UUID REFERENCES public.households(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('chequing', 'savings', 'credit_card', 'investment', 'other')),
  balance DECIMAL(10, 2) DEFAULT 0,
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
  updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- Create index for better query performance
CREATE INDEX idx_accounts_user_id ON public.accounts(user_id);
CREATE INDEX idx_accounts_household_id ON public.accounts(household_id);

-- Enable Row Level Security
ALTER TABLE public.accounts ENABLE ROW LEVEL SECURITY;

-- RLS Policies
CREATE POLICY "Users can view their own accounts"
  ON public.accounts FOR SELECT
  USING (user_id = auth.uid() OR household_id IN (SELECT household_id FROM public.household_members WHERE user_id = auth.uid()));

CREATE POLICY "Users can create their own accounts"
  ON public.accounts FOR INSERT
  WITH CHECK (user_id = auth.uid());

CREATE POLICY "Users can update their own accounts"
  ON public.accounts FOR UPDATE
  USING (user_id = auth.uid());

CREATE POLICY "Users can delete their own accounts"
  ON public.accounts FOR DELETE
  USING (user_id = auth.uid());

-- Add account_id to expenses table
ALTER TABLE public.expenses
ADD COLUMN account_id UUID REFERENCES public.accounts(id) ON DELETE SET NULL;

CREATE INDEX idx_expenses_account_id ON public.expenses(account_id);


