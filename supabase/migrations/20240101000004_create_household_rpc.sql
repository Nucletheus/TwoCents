-- Atomic household creation that works with RLS:
-- creates a household + inserts the creator as owner in household_members, then returns the household.

CREATE OR REPLACE FUNCTION public.create_household(p_name text)
RETURNS public.households
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  v_household public.households;
  v_uid uuid;
BEGIN
  v_uid := auth.uid();
  IF v_uid IS NULL THEN
    RAISE EXCEPTION 'Not authenticated';
  END IF;

  INSERT INTO public.households (name)
  VALUES (p_name)
  RETURNING * INTO v_household;

  INSERT INTO public.household_members (household_id, user_id, role)
  VALUES (v_household.id, v_uid, 'owner');

  RETURN v_household;
END;
$$;

REVOKE ALL ON FUNCTION public.create_household(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.create_household(text) TO authenticated;


