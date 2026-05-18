-- Household-specific overrides for category group colors.
-- This lets each household choose a base color per `group_name` without modifying default categories.

CREATE TABLE IF NOT EXISTS public.household_category_group_colors (
  household_id UUID NOT NULL REFERENCES public.households(id) ON DELETE CASCADE,
  group_name TEXT NOT NULL,
  color TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (household_id, group_name)
);

-- RLS
ALTER TABLE public.household_category_group_colors ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view household category group colors in their households"
  ON public.household_category_group_colors FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_category_group_colors.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can insert household category group colors in their households"
  ON public.household_category_group_colors FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_category_group_colors.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update household category group colors in their households"
  ON public.household_category_group_colors FOR UPDATE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_category_group_colors.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete household category group colors in their households"
  ON public.household_category_group_colors FOR DELETE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_category_group_colors.household_id
      AND hm.user_id = auth.uid()
    )
  );


