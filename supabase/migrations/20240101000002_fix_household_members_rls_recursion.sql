-- Fix "infinite recursion detected in policy for relation household_members"
-- by avoiding policies that query household_members through RLS.
--
-- Approach:
-- - Add SECURITY DEFINER helper functions (table owner bypasses RLS unless FORCE RLS is enabled)
-- - Recreate household_members and households policies to use these helpers

-- Helper: is current user a member of a household?
CREATE OR REPLACE FUNCTION public.is_household_member(_household_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
  SELECT EXISTS (
    SELECT 1
    FROM public.household_members hm
    WHERE hm.household_id = _household_id
      AND hm.user_id = auth.uid()
  );
$$;

REVOKE ALL ON FUNCTION public.is_household_member(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.is_household_member(uuid) TO authenticated;

-- Helper: is current user an owner of a household?
CREATE OR REPLACE FUNCTION public.is_household_owner(_household_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
  SELECT EXISTS (
    SELECT 1
    FROM public.household_members hm
    WHERE hm.household_id = _household_id
      AND hm.user_id = auth.uid()
      AND hm.role = 'owner'
  );
$$;

REVOKE ALL ON FUNCTION public.is_household_owner(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.is_household_owner(uuid) TO authenticated;

-- Recreate households policies to use helpers (avoids dependency on household_members RLS)
DROP POLICY IF EXISTS "Users can view households they belong to" ON public.households;
CREATE POLICY "Users can view households they belong to"
  ON public.households FOR SELECT
  USING (public.is_household_member(id));

DROP POLICY IF EXISTS "Owners can update their households" ON public.households;
CREATE POLICY "Owners can update their households"
  ON public.households FOR UPDATE
  USING (public.is_household_owner(id));

-- Recreate household_members policies (fixes recursion + fixes INSERT check bug)
DROP POLICY IF EXISTS "Users can view household members of their households" ON public.household_members;
CREATE POLICY "Users can view household members of their households"
  ON public.household_members FOR SELECT
  USING (public.is_household_member(household_id));

DROP POLICY IF EXISTS "Owners can add members to their households" ON public.household_members;
CREATE POLICY "Owners can add members to their households"
  ON public.household_members FOR INSERT
  WITH CHECK (public.is_household_owner(household_id));


