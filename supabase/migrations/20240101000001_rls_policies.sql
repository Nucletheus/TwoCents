-- Enable Row Level Security on all tables
ALTER TABLE public.profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.households ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.household_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.categories ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.expenses ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.expense_splits ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.savings_goals ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.goal_contributions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.budgets ENABLE ROW LEVEL SECURITY;

-- Profiles policies
CREATE POLICY "Users can view their own profile"
  ON public.profiles FOR SELECT
  USING (auth.uid() = id);

CREATE POLICY "Users can update their own profile"
  ON public.profiles FOR UPDATE
  USING (auth.uid() = id);

-- Households policies
CREATE POLICY "Users can view households they belong to"
  ON public.households FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = households.id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create households"
  ON public.households FOR INSERT
  WITH CHECK (true);

CREATE POLICY "Owners can update their households"
  ON public.households FOR UPDATE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = households.id
      AND household_members.user_id = auth.uid()
      AND household_members.role = 'owner'
    )
  );

-- Household members policies
CREATE POLICY "Users can view household members of their households"
  ON public.household_members FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_members.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Owners can add members to their households"
  ON public.household_members FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = household_members.household_id
      AND household_members.user_id = auth.uid()
      AND household_members.role = 'owner'
    )
  );

-- Categories policies
CREATE POLICY "Users can view categories in their households or default categories"
  ON public.categories FOR SELECT
  USING (
    is_default = true
    OR EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = categories.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create categories in their households"
  ON public.categories FOR INSERT
  WITH CHECK (
    household_id IS NULL
    OR EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = categories.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update categories in their households"
  ON public.categories FOR UPDATE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = categories.household_id
      AND household_members.user_id = auth.uid()
    )
  );

-- Expenses policies
CREATE POLICY "Users can view expenses in their households"
  ON public.expenses FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = expenses.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create expenses in their households"
  ON public.expenses FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = expenses.household_id
      AND household_members.user_id = auth.uid()
    )
    AND payer_id = auth.uid()
  );

CREATE POLICY "Users can update expenses they paid for"
  ON public.expenses FOR UPDATE
  USING (
    payer_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = expenses.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete expenses they paid for"
  ON public.expenses FOR DELETE
  USING (
    payer_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = expenses.household_id
      AND household_members.user_id = auth.uid()
    )
  );

-- Expense splits policies
CREATE POLICY "Users can view expense splits in their households"
  ON public.expense_splits FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.expenses e
      JOIN public.household_members hm ON hm.household_id = e.household_id
      WHERE e.id = expense_splits.expense_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create expense splits for expenses in their households"
  ON public.expense_splits FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.expenses e
      JOIN public.household_members hm ON hm.household_id = e.household_id
      WHERE e.id = expense_splits.expense_id
      AND hm.user_id = auth.uid()
    )
  );

-- Savings goals policies
CREATE POLICY "Users can view savings goals in their households"
  ON public.savings_goals FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = savings_goals.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create savings goals in their households"
  ON public.savings_goals FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = savings_goals.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update savings goals in their households"
  ON public.savings_goals FOR UPDATE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = savings_goals.household_id
      AND household_members.user_id = auth.uid()
    )
  );

-- Goal contributions policies
CREATE POLICY "Users can view goal contributions in their households"
  ON public.goal_contributions FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.savings_goals sg
      JOIN public.household_members hm ON hm.household_id = sg.household_id
      WHERE sg.id = goal_contributions.goal_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create goal contributions for goals in their households"
  ON public.goal_contributions FOR INSERT
  WITH CHECK (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.savings_goals sg
      JOIN public.household_members hm ON hm.household_id = sg.household_id
      WHERE sg.id = goal_contributions.goal_id
      AND hm.user_id = auth.uid()
    )
  );

-- Budgets policies
CREATE POLICY "Users can view budgets in their households"
  ON public.budgets FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = budgets.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create budgets in their households"
  ON public.budgets FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = budgets.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update budgets in their households"
  ON public.budgets FOR UPDATE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = budgets.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete budgets in their households"
  ON public.budgets FOR DELETE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = budgets.household_id
      AND household_members.user_id = auth.uid()
    )
  );

