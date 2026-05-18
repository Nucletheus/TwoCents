-- Allow household members to create expenses for other household members.
-- This fixes the previous policy which required payer_id = auth.uid(), preventing
-- household members from importing or creating expenses on behalf of other members.
--
-- This is necessary for CSV imports where one member imports transactions for another member.

-- Drop old INSERT policy
DROP POLICY IF EXISTS "Users can create expenses in their households" ON public.expenses;

-- INSERT: household members can create expenses in their household for any member of that household
CREATE POLICY "Household members can create expenses in their households"
  ON public.expenses FOR INSERT
  WITH CHECK (
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

