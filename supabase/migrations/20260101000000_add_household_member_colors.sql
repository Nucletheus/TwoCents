-- Create household_member_colors table for storing user-assigned colors per household
CREATE TABLE IF NOT EXISTS public.household_member_colors (
  household_id UUID NOT NULL REFERENCES public.households(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  color TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (household_id, user_id)
);

-- Create index for better query performance
CREATE INDEX IF NOT EXISTS idx_household_member_colors_household_id ON public.household_member_colors(household_id);
CREATE INDEX IF NOT EXISTS idx_household_member_colors_user_id ON public.household_member_colors(user_id);

-- Enable Row Level Security
ALTER TABLE public.household_member_colors ENABLE ROW LEVEL SECURITY;

-- RLS Policies
CREATE POLICY "Users can view household member colors in their households"
  ON public.household_member_colors FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_member_colors.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can insert their own household member colors"
  ON public.household_member_colors FOR INSERT
  WITH CHECK (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_member_colors.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update their own household member colors"
  ON public.household_member_colors FOR UPDATE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_member_colors.household_id
      AND hm.user_id = auth.uid()
    )
  )
  WITH CHECK (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_member_colors.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete their own household member colors"
  ON public.household_member_colors FOR DELETE
  USING (
    user_id = auth.uid()
    AND EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = household_member_colors.household_id
      AND hm.user_id = auth.uid()
    )
  );

