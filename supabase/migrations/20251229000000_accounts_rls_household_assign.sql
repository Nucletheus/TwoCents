-- Allow household members to create/update/delete accounts within their household,
-- and assign account ownership (user_id) to any member of the same household.
--
-- This fixes the previous policies which required user_id = auth.uid(), preventing
-- assigning accounts to other household members.

-- Drop old policies (created in 20240101000008_create_accounts.sql)
DROP POLICY IF EXISTS "Users can view their own accounts" ON public.accounts;
DROP POLICY IF EXISTS "Users can create their own accounts" ON public.accounts;
DROP POLICY IF EXISTS "Users can update their own accounts" ON public.accounts;
DROP POLICY IF EXISTS "Users can delete their own accounts" ON public.accounts;

-- SELECT: household members can view all accounts in their household; users can also view their own personal accounts
CREATE POLICY "Household members can view accounts"
  ON public.accounts
  FOR SELECT
  USING (
    (household_id IS NULL AND user_id = auth.uid())
    OR EXISTS (
      SELECT 1
      FROM public.household_members hm
      WHERE hm.household_id = accounts.household_id
        AND hm.user_id = auth.uid()
    )
  );

-- INSERT: household members can create accounts for their household, and may assign user_id to any member of that household.
-- For personal accounts (household_id IS NULL), user_id must be the current user.
CREATE POLICY "Household members can create accounts"
  ON public.accounts
  FOR INSERT
  WITH CHECK (
    (household_id IS NULL AND user_id = auth.uid())
    OR (
      EXISTS (
        SELECT 1
        FROM public.household_members hm
        WHERE hm.household_id = accounts.household_id
          AND hm.user_id = auth.uid()
      )
      AND EXISTS (
        SELECT 1
        FROM public.household_members hm2
        WHERE hm2.household_id = accounts.household_id
          AND hm2.user_id = accounts.user_id
      )
    )
  );

-- UPDATE: household members can update accounts in their household.
-- Any updated user_id must remain a member of the same household.
CREATE POLICY "Household members can update accounts"
  ON public.accounts
  FOR UPDATE
  USING (
    (household_id IS NULL AND user_id = auth.uid())
    OR EXISTS (
      SELECT 1
      FROM public.household_members hm
      WHERE hm.household_id = accounts.household_id
        AND hm.user_id = auth.uid()
    )
  )
  WITH CHECK (
    (household_id IS NULL AND user_id = auth.uid())
    OR (
      EXISTS (
        SELECT 1
        FROM public.household_members hm
        WHERE hm.household_id = accounts.household_id
          AND hm.user_id = auth.uid()
      )
      AND EXISTS (
        SELECT 1
        FROM public.household_members hm2
        WHERE hm2.household_id = accounts.household_id
          AND hm2.user_id = accounts.user_id
      )
    )
  );

-- DELETE: household members can delete accounts in their household; users can delete their own personal accounts
CREATE POLICY "Household members can delete accounts"
  ON public.accounts
  FOR DELETE
  USING (
    (household_id IS NULL AND user_id = auth.uid())
    OR EXISTS (
      SELECT 1
      FROM public.household_members hm
      WHERE hm.household_id = accounts.household_id
        AND hm.user_id = auth.uid()
    )
  );


