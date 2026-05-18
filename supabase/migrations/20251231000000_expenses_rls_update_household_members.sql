-- Allow household members to update expenses for other household members.
-- This fixes the previous policy which required payer_id = auth.uid(), preventing
-- household members from updating expenses on behalf of other members.
--
-- This is necessary for categorizing and managing expenses in shared households.

-- Drop old UPDATE policy
DROP POLICY IF EXISTS "Users can update expenses they paid for" ON public.expenses;

-- UPDATE: household members can update expenses in their household for any member of that household
CREATE POLICY "Household members can update expenses in their households"
  ON public.expenses FOR UPDATE
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

