-- Allow household members to view each other's profile rows (e.g., names) for collaboration features.
-- Without this, joining household_members -> profiles will only return the current user's profile.

DROP POLICY IF EXISTS "Users can view profiles in their households" ON public.profiles;

CREATE POLICY "Users can view profiles in their households"
  ON public.profiles FOR SELECT
  USING (
    EXISTS (
      SELECT 1
      FROM public.household_members hm_self
      JOIN public.household_members hm_other
        ON hm_other.household_id = hm_self.household_id
      WHERE hm_self.user_id = auth.uid()
        AND hm_other.user_id = profiles.id
    )
  );


