-- Create household_income table for storing annual income per household member per year
CREATE TABLE IF NOT EXISTS public.household_income (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  household_id UUID NOT NULL REFERENCES public.households(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  annual_income DECIMAL(10, 2) NOT NULL CHECK (annual_income >= 0),
  year INTEGER NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (household_id, user_id, year)
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_household_income_household_id ON public.household_income(household_id);
CREATE INDEX IF NOT EXISTS idx_household_income_user_id ON public.household_income(user_id);
CREATE INDEX IF NOT EXISTS idx_household_income_year ON public.household_income(year);

-- Enable Row Level Security
ALTER TABLE public.household_income ENABLE ROW LEVEL SECURITY;

-- RLS Policies
CREATE POLICY "Users can view household income in their households"
  ON public.household_income FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_income.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create household income in their households"
  ON public.household_income FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_income.household_id
      AND hm.user_id = auth.uid()
    )
    AND user_id = auth.uid()
  );

CREATE POLICY "Users can update their own household income"
  ON public.household_income FOR UPDATE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_income.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete their own household income"
  ON public.household_income FOR DELETE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_income.household_id
      AND hm.user_id = auth.uid()
    )
  );

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_household_income_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to automatically update updated_at
CREATE TRIGGER household_income_updated_at
  BEFORE UPDATE ON public.household_income
  FOR EACH ROW
  EXECUTE FUNCTION update_household_income_updated_at();

