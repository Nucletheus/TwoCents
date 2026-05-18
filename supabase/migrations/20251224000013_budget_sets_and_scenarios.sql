-- Add budget sets (main + scenarios) and attach budgets to a budget_set_id

-- 1) Budget sets
CREATE TABLE IF NOT EXISTS public.budget_sets (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  household_id UUID NOT NULL REFERENCES public.households(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  is_main BOOLEAN NOT NULL DEFAULT false,
  created_by UUID REFERENCES auth.users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One main budget set per household
CREATE UNIQUE INDEX IF NOT EXISTS budget_sets_one_main_per_household
  ON public.budget_sets (household_id)
  WHERE is_main = true;

CREATE INDEX IF NOT EXISTS idx_budget_sets_household_id
  ON public.budget_sets (household_id);

-- RLS
ALTER TABLE public.budget_sets ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view budget sets in their households"
  ON public.budget_sets FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = budget_sets.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create budget sets in their households"
  ON public.budget_sets FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = budget_sets.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update budget sets in their households"
  ON public.budget_sets FOR UPDATE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = budget_sets.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete budget sets in their households"
  ON public.budget_sets FOR DELETE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = budget_sets.household_id
      AND hm.user_id = auth.uid()
    )
  );

-- 2) Attach existing budgets to a budget_set_id
ALTER TABLE public.budgets
ADD COLUMN IF NOT EXISTS budget_set_id UUID REFERENCES public.budget_sets(id) ON DELETE CASCADE;

-- Create a main budget set per household (backfill)
INSERT INTO public.budget_sets (household_id, name, is_main, created_by)
SELECT h.id, 'Main Budget', true, NULL
FROM public.households h
ON CONFLICT DO NOTHING;

-- Backfill budget_set_id for existing budgets to the household's main budget set
UPDATE public.budgets b
SET budget_set_id = bs.id
FROM public.budget_sets bs
WHERE bs.household_id = b.household_id
  AND bs.is_main = true
  AND b.budget_set_id IS NULL;

-- Require budget_set_id going forward
ALTER TABLE public.budgets
ALTER COLUMN budget_set_id SET NOT NULL;

-- Replace uniqueness: allow multiple sets per household, still unique per set/category/period
ALTER TABLE public.budgets
DROP CONSTRAINT IF EXISTS budgets_household_id_category_id_period_key;

ALTER TABLE public.budgets
ADD CONSTRAINT budgets_budget_set_id_category_id_period_key
UNIQUE (budget_set_id, category_id, period);

CREATE INDEX IF NOT EXISTS idx_budgets_budget_set_id
  ON public.budgets (budget_set_id);

-- 3) Ensure new households get a main budget set
-- NOTE: this overrides the original create_household() function after budget_sets exists.
CREATE OR REPLACE FUNCTION public.create_household(p_name text)
RETURNS public.households
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  v_household public.households;
  v_uid uuid;
BEGIN
  v_uid := auth.uid();
  IF v_uid IS NULL THEN
    RAISE EXCEPTION 'Not authenticated';
  END IF;

  INSERT INTO public.households (name)
  VALUES (p_name)
  RETURNING * INTO v_household;

  INSERT INTO public.household_members (household_id, user_id, role)
  VALUES (v_household.id, v_uid, 'owner');

  INSERT INTO public.budget_sets (household_id, name, is_main, created_by)
  VALUES (v_household.id, 'Main Budget', true, v_uid)
  ON CONFLICT DO NOTHING;

  RETURN v_household;
END;
$$;

REVOKE ALL ON FUNCTION public.create_household(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.create_household(text) TO authenticated;


