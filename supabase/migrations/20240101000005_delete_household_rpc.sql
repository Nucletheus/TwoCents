-- Delete household function with proper RLS checks
-- Only owners can delete their households
-- Cascade deletes are handled by foreign key constraints

CREATE OR REPLACE FUNCTION public.delete_household(p_household_id uuid)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  v_uid uuid;
BEGIN
  v_uid := auth.uid();
  IF v_uid IS NULL THEN
    RAISE EXCEPTION 'Not authenticated';
  END IF;

  -- Check if user is owner of the household
  IF NOT EXISTS (
    SELECT 1
    FROM public.household_members
    WHERE household_id = p_household_id
      AND user_id = v_uid
      AND role = 'owner'
  ) THEN
    RAISE EXCEPTION 'Only household owners can delete households';
  END IF;

  -- Delete the household (cascade deletes will handle related records)
  DELETE FROM public.households
  WHERE id = p_household_id;
END;
$$;

REVOKE ALL ON FUNCTION public.delete_household(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.delete_household(uuid) TO authenticated;

