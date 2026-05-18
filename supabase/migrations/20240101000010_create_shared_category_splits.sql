-- Create shared_category_splits to store per-member shared category splits
CREATE TABLE public.shared_category_splits (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  household_id UUID NOT NULL REFERENCES public.households(id) ON DELETE CASCADE,
  category_id UUID NOT NULL REFERENCES public.categories(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  percentage DECIMAL(5, 2) NOT NULL CHECK (percentage >= 0 AND percentage <= 100),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (household_id, category_id, user_id)
);

-- Helpful indexes
CREATE INDEX idx_shared_category_splits_household ON public.shared_category_splits (household_id);
CREATE INDEX idx_shared_category_splits_category ON public.shared_category_splits (category_id);

-- RLS
ALTER TABLE public.shared_category_splits ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view shared category splits in their households"
  ON public.shared_category_splits FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = shared_category_splits.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can insert shared category splits for their households"
  ON public.shared_category_splits FOR INSERT
  WITH CHECK (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = shared_category_splits.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update their shared category splits in their households"
  ON public.shared_category_splits FOR UPDATE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = shared_category_splits.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete their shared category splits in their households"
  ON public.shared_category_splits FOR DELETE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = shared_category_splits.household_id
      AND hm.user_id = auth.uid()
    )
  );

