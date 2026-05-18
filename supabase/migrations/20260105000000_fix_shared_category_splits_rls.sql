-- Fix RLS policies for shared_category_splits to allow household members
-- to manage splits for any member of their household, not just themselves.
-- This is necessary because when configuring shared categories, one member
-- needs to set up splits for all household members at once.

-- Drop old policies
DROP POLICY IF EXISTS "Users can insert shared category splits for their households" ON public.shared_category_splits;
DROP POLICY IF EXISTS "Users can update their shared category splits in their households" ON public.shared_category_splits;
DROP POLICY IF EXISTS "Users can delete their shared category splits in their households" ON public.shared_category_splits;

-- INSERT: household members can insert splits for any member of their household
CREATE POLICY "Household members can insert shared category splits"
  ON public.shared_category_splits FOR INSERT
  WITH CHECK (
    -- Current user must be a member of the household
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = shared_category_splits.household_id
      AND hm.user_id = auth.uid()
    )
    -- Target user must also be a member of the same household
    AND EXISTS (
      SELECT 1 FROM public.household_members hm2
      WHERE hm2.household_id = shared_category_splits.household_id
      AND hm2.user_id = shared_category_splits.user_id
    )
  );

-- UPDATE: household members can update splits for any member of their household
CREATE POLICY "Household members can update shared category splits"
  ON public.shared_category_splits FOR UPDATE
  USING (
    -- Current user must be a member of the household
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = shared_category_splits.household_id
      AND hm.user_id = auth.uid()
    )
    -- Target user must also be a member of the same household
    AND EXISTS (
      SELECT 1 FROM public.household_members hm2
      WHERE hm2.household_id = shared_category_splits.household_id
      AND hm2.user_id = shared_category_splits.user_id
    )
  )
  WITH CHECK (
    -- Same checks for the new values
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = shared_category_splits.household_id
      AND hm.user_id = auth.uid()
    )
    AND EXISTS (
      SELECT 1 FROM public.household_members hm2
      WHERE hm2.household_id = shared_category_splits.household_id
      AND hm2.user_id = shared_category_splits.user_id
    )
  );

-- DELETE: household members can delete splits for any member of their household
CREATE POLICY "Household members can delete shared category splits"
  ON public.shared_category_splits FOR DELETE
  USING (
    -- Current user must be a member of the household
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = shared_category_splits.household_id
      AND hm.user_id = auth.uid()
    )
    -- Target user must also be a member of the same household
    AND EXISTS (
      SELECT 1 FROM public.household_members hm2
      WHERE hm2.household_id = shared_category_splits.household_id
      AND hm2.user_id = shared_category_splits.user_id
    )
  );

