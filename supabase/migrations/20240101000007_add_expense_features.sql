-- Add expense features: transaction rules, review flags, CSV presets, and collaboration

-- Add new fields to expenses table
ALTER TABLE public.expenses
ADD COLUMN vendor TEXT,
ADD COLUMN status TEXT DEFAULT 'approved' CHECK (status IN ('pending_review', 'approved', 'flagged')),
ADD COLUMN reviewed_by UUID REFERENCES auth.users(id) ON DELETE SET NULL,
ADD COLUMN reviewed_at TIMESTAMPTZ;

-- Add parent_color to categories table for color grouping
ALTER TABLE public.categories
ADD COLUMN parent_color TEXT;

-- Create transaction_rules table
CREATE TABLE public.transaction_rules (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  household_id UUID REFERENCES public.households(id) ON DELETE CASCADE NOT NULL,
  user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  name TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  match_type TEXT NOT NULL CHECK (match_type IN ('vendor', 'description', 'amount_range', 'date_pattern', 'combination')),
  match_pattern JSONB NOT NULL,
  category_id UUID REFERENCES public.categories(id) ON DELETE SET NULL,
  is_active BOOLEAN DEFAULT true NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- Create expense_flags table
CREATE TABLE public.expense_flags (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  expense_id UUID REFERENCES public.expenses(id) ON DELETE CASCADE NOT NULL,
  user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  flag_type TEXT NOT NULL CHECK (flag_type IN ('review', 'shared', 'needs_attention')),
  notes TEXT,
  resolved_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- Create vendor_review_flags table
CREATE TABLE public.vendor_review_flags (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  household_id UUID REFERENCES public.households(id) ON DELETE CASCADE NOT NULL,
  vendor_name TEXT NOT NULL,
  requires_review BOOLEAN DEFAULT true NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
  UNIQUE (household_id, vendor_name)
);

-- Create csv_import_presets table
CREATE TABLE public.csv_import_presets (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  household_id UUID REFERENCES public.households(id) ON DELETE CASCADE NOT NULL,
  preset_name TEXT NOT NULL,
  filename_pattern TEXT,
  column_mapping JSONB NOT NULL,
  default_payer_id UUID REFERENCES auth.users(id) ON DELETE SET NULL,
  default_category_id UUID REFERENCES public.categories(id) ON DELETE SET NULL,
  skip_columns JSONB DEFAULT '[]'::jsonb,
  last_used_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- Create expense_comments table
CREATE TABLE public.expense_comments (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  expense_id UUID REFERENCES public.expenses(id) ON DELETE CASCADE NOT NULL,
  user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  comment TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- Create indexes for performance
CREATE INDEX idx_expenses_household_status ON public.expenses(household_id, status);
CREATE INDEX idx_expenses_household_date_status ON public.expenses(household_id, date, status);
CREATE INDEX idx_expenses_household_payer_date ON public.expenses(household_id, payer_id, date);
CREATE INDEX idx_expenses_vendor ON public.expenses(vendor);
CREATE INDEX idx_expenses_status ON public.expenses(status);

CREATE INDEX idx_transaction_rules_household ON public.transaction_rules(household_id);
CREATE INDEX idx_transaction_rules_active ON public.transaction_rules(is_active, priority);

CREATE INDEX idx_expense_flags_expense ON public.expense_flags(expense_id);
CREATE INDEX idx_expense_flags_user ON public.expense_flags(user_id);
CREATE INDEX idx_expense_flags_type ON public.expense_flags(flag_type);
CREATE INDEX idx_expense_flags_resolved ON public.expense_flags(resolved_at) WHERE resolved_at IS NULL;

CREATE INDEX idx_vendor_review_flags_household ON public.vendor_review_flags(household_id);
CREATE INDEX idx_vendor_review_flags_vendor ON public.vendor_review_flags(vendor_name);

CREATE INDEX idx_csv_import_presets_user ON public.csv_import_presets(user_id);
CREATE INDEX idx_csv_import_presets_household ON public.csv_import_presets(household_id);
CREATE INDEX idx_csv_import_presets_pattern ON public.csv_import_presets(filename_pattern) WHERE filename_pattern IS NOT NULL;

CREATE INDEX idx_expense_comments_expense ON public.expense_comments(expense_id);
CREATE INDEX idx_expense_comments_user ON public.expense_comments(user_id);

-- RLS Policies for transaction_rules
ALTER TABLE public.transaction_rules ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view rules in their households"
  ON public.transaction_rules FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = transaction_rules.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create rules in their households"
  ON public.transaction_rules FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = transaction_rules.household_id
      AND household_members.user_id = auth.uid()
    )
    AND user_id = auth.uid()
  );

CREATE POLICY "Users can update their own rules"
  ON public.transaction_rules FOR UPDATE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = transaction_rules.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete their own rules"
  ON public.transaction_rules FOR DELETE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = transaction_rules.household_id
      AND household_members.user_id = auth.uid()
    )
  );

-- RLS Policies for expense_flags
ALTER TABLE public.expense_flags ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view flags for expenses in their households"
  ON public.expense_flags FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.expenses
      JOIN public.household_members ON household_members.household_id = expenses.household_id
      WHERE expenses.id = expense_flags.expense_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create flags for expenses in their households"
  ON public.expense_flags FOR INSERT
  WITH CHECK (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.expenses
      JOIN public.household_members ON household_members.household_id = expenses.household_id
      WHERE expenses.id = expense_flags.expense_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update flags for expenses in their households"
  ON public.expense_flags FOR UPDATE
  USING (
    EXISTS (
      SELECT 1 FROM public.expenses
      JOIN public.household_members ON household_members.household_id = expenses.household_id
      WHERE expenses.id = expense_flags.expense_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete flags for expenses in their households"
  ON public.expense_flags FOR DELETE
  USING (
    EXISTS (
      SELECT 1 FROM public.expenses
      JOIN public.household_members ON household_members.household_id = expenses.household_id
      WHERE expenses.id = expense_flags.expense_id
      AND household_members.user_id = auth.uid()
    )
  );

-- RLS Policies for vendor_review_flags
ALTER TABLE public.vendor_review_flags ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view vendor flags in their households"
  ON public.vendor_review_flags FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = vendor_review_flags.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can manage vendor flags in their households"
  ON public.vendor_review_flags FOR ALL
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = vendor_review_flags.household_id
      AND household_members.user_id = auth.uid()
    )
  );

-- RLS Policies for csv_import_presets
ALTER TABLE public.csv_import_presets ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view their own presets"
  ON public.csv_import_presets FOR SELECT
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = csv_import_presets.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create their own presets"
  ON public.csv_import_presets FOR INSERT
  WITH CHECK (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = csv_import_presets.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update their own presets"
  ON public.csv_import_presets FOR UPDATE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = csv_import_presets.household_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete their own presets"
  ON public.csv_import_presets FOR DELETE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = csv_import_presets.household_id
      AND household_members.user_id = auth.uid()
    )
  );

-- RLS Policies for expense_comments
ALTER TABLE public.expense_comments ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view comments for expenses in their households"
  ON public.expense_comments FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.expenses
      JOIN public.household_members ON household_members.household_id = expenses.household_id
      WHERE expenses.id = expense_comments.expense_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create comments for expenses in their households"
  ON public.expense_comments FOR INSERT
  WITH CHECK (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.expenses
      JOIN public.household_members ON household_members.household_id = expenses.household_id
      WHERE expenses.id = expense_comments.expense_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update their own comments"
  ON public.expense_comments FOR UPDATE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.expenses
      JOIN public.household_members ON household_members.household_id = expenses.household_id
      WHERE expenses.id = expense_comments.expense_id
      AND household_members.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete their own comments"
  ON public.expense_comments FOR DELETE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.expenses
      JOIN public.household_members ON household_members.household_id = expenses.household_id
      WHERE expenses.id = expense_comments.expense_id
      AND household_members.user_id = auth.uid()
    )
  );

-- Function to extract vendor from description (can be called on insert/update)
CREATE OR REPLACE FUNCTION public.extract_vendor_from_description(description_text TEXT)
RETURNS TEXT AS $$
DECLARE
  vendor_name TEXT;
BEGIN
  -- Simple extraction: take first part before common separators
  -- This is a basic implementation, can be enhanced
  vendor_name := regexp_replace(description_text, '^([^|#\-]+).*', '\1');
  vendor_name := trim(vendor_name);
  RETURN vendor_name;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Trigger to auto-extract vendor on expense insert/update
CREATE OR REPLACE FUNCTION public.auto_extract_vendor()
RETURNS TRIGGER AS $$
BEGIN
  IF NEW.description IS NOT NULL AND (NEW.vendor IS NULL OR NEW.vendor = '') THEN
    NEW.vendor := public.extract_vendor_from_description(NEW.description);
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER extract_vendor_trigger
  BEFORE INSERT OR UPDATE OF description ON public.expenses
  FOR EACH ROW
  EXECUTE FUNCTION public.auto_extract_vendor();

