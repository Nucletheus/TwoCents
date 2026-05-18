-- Prevent clients from inserting "template" categories (household_id NULL).
-- Default categories are templates and should only be managed by migrations/service role.

DROP POLICY IF EXISTS "Users can create categories in their households" ON public.categories;

CREATE POLICY "Users can create categories in their households"
  ON public.categories FOR INSERT
  WITH CHECK (
    household_id IS NOT NULL
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = categories.household_id
        AND household_members.user_id = auth.uid()
    )
  );


