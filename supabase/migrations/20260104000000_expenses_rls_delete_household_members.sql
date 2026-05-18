-- Allow household members to delete expenses for other household members.
-- This fixes the previous policy which required payer_id = auth.uid(), preventing
-- household members from deleting expenses on behalf of other members.
--
-- This is necessary for bulk deletion operations in shared households where
-- one member may need to delete expenses paid by another member.

-- Drop old DELETE policy
DROP POLICY IF EXISTS "Users can delete expenses they paid for" ON public.expenses;

-- DELETE: household members can delete expenses in their household for any member of that household
CREATE POLICY "Household members can delete expenses in their households"
  ON public.expenses FOR DELETE
  USING (
    -- Current user must be a member of the household
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = expenses.household_id
      AND hm.user_id = auth.uid()
    )
    -- Payer must also be a member of the same household
    AND EXISTS (
      SELECT 1 FROM public.household_members hm2
      WHERE hm2.household_id = expenses.household_id
      AND hm2.user_id = expenses.payer_id
    )
  );

