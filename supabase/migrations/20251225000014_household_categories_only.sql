-- Make categories household-specific: default categories are templates only.
-- 1) Ensure each household has its own copy of default categories
-- 2) Remap references away from default category IDs
-- 3) Update create_household() to copy defaults into the new household
-- 4) Tighten categories SELECT policy so clients only see household categories

-- 1) Backfill household categories from templates (public.categories where is_default=true)
INSERT INTO public.categories (
  name,
  icon,
  color,
  group_name,
  parent_color,
  household_id,
  is_default
)
SELECT
  d.name,
  d.icon,
  d.color,
  d.group_name,
  d.parent_color,
  h.id AS household_id,
  false AS is_default
FROM public.households h
JOIN public.categories d ON d.is_default = true AND d.household_id IS NULL
LEFT JOIN public.categories hc
  ON hc.household_id = h.id
 AND hc.name = d.name
 AND hc.group_name IS NOT DISTINCT FROM d.group_name
WHERE hc.id IS NULL;

-- 2) Remap references to household category IDs (match by household_id + name + group_name)
-- Expenses
UPDATE public.expenses e
SET category_id = hc.id
FROM public.categories d
JOIN public.categories hc
  ON hc.name = d.name
 AND hc.group_name IS NOT DISTINCT FROM d.group_name
WHERE e.category_id = d.id
  AND d.is_default = true
  AND hc.household_id = e.household_id;

-- Budgets
-- Collision-safe remap (budgets has a unique constraint on budget_set_id/category_id/period)
-- If a budget already exists for the household category, keep it and drop the default-category row.
UPDATE public.budgets existing
SET amount = GREATEST(existing.amount, b.amount)
FROM public.budgets b,
     public.categories d,
     public.categories hc
WHERE b.category_id = d.id
  AND d.is_default = true
  AND hc.name = d.name
  AND hc.group_name IS NOT DISTINCT FROM d.group_name
  AND hc.household_id = b.household_id
  AND existing.budget_set_id = b.budget_set_id
  AND existing.period = b.period
  AND existing.category_id = hc.id;

DELETE FROM public.budgets b
USING public.categories d, public.categories hc, public.budgets existing
WHERE b.category_id = d.id
  AND d.is_default = true
  AND hc.name = d.name
  AND hc.group_name IS NOT DISTINCT FROM d.group_name
  AND hc.household_id = b.household_id
  AND existing.budget_set_id = b.budget_set_id
  AND existing.period = b.period
  AND existing.category_id = hc.id;

UPDATE public.budgets b
SET category_id = hc.id
FROM public.categories d, public.categories hc
WHERE b.category_id = d.id
  AND d.is_default = true
  AND hc.name = d.name
  AND hc.group_name IS NOT DISTINCT FROM d.group_name
  AND hc.household_id = b.household_id
  AND NOT EXISTS (
    SELECT 1 FROM public.budgets existing
    WHERE existing.budget_set_id = b.budget_set_id
      AND existing.period = b.period
      AND existing.category_id = hc.id
  );

-- Transaction rules
UPDATE public.transaction_rules tr
SET category_id = hc.id
FROM public.categories d
JOIN public.categories hc
  ON hc.name = d.name
 AND hc.group_name IS NOT DISTINCT FROM d.group_name
WHERE tr.category_id = d.id
  AND d.is_default = true
  AND hc.household_id = tr.household_id;

-- Shared category splits
-- Collision-safe remap (unique on household_id/category_id/user_id)
DELETE FROM public.shared_category_splits scs
USING public.categories d, public.categories hc, public.shared_category_splits existing
WHERE scs.category_id = d.id
  AND d.is_default = true
  AND hc.name = d.name
  AND hc.group_name IS NOT DISTINCT FROM d.group_name
  AND hc.household_id = scs.household_id
  AND existing.household_id = scs.household_id
  AND existing.user_id = scs.user_id
  AND existing.category_id = hc.id;

UPDATE public.shared_category_splits scs
SET category_id = hc.id
FROM public.categories d, public.categories hc
WHERE scs.category_id = d.id
  AND d.is_default = true
  AND hc.name = d.name
  AND hc.group_name IS NOT DISTINCT FROM d.group_name
  AND hc.household_id = scs.household_id
  AND NOT EXISTS (
    SELECT 1 FROM public.shared_category_splits existing
    WHERE existing.household_id = scs.household_id
      AND existing.user_id = scs.user_id
      AND existing.category_id = hc.id
  );

-- CSV presets default category
UPDATE public.csv_import_presets p
SET default_category_id = hc.id
FROM public.categories d
JOIN public.categories hc
  ON hc.name = d.name
 AND hc.group_name IS NOT DISTINCT FROM d.group_name
WHERE p.default_category_id = d.id
  AND d.is_default = true
  AND hc.household_id = p.household_id;

-- 3) Update create_household() to also copy template categories
-- NOTE: This overrides the previous version (budget_sets + membership creation).
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

  INSERT INTO public.budget_sets (household_id, name, is_main, created_by)
  VALUES (v_household.id, 'Main Budget', true, v_uid)
  ON CONFLICT DO NOTHING;

  -- Copy template categories into this household.
  INSERT INTO public.categories (
    name,
    icon,
    color,
    group_name,
    parent_color,
    household_id,
    is_default
  )
  SELECT
    d.name,
    d.icon,
    d.color,
    d.group_name,
    d.parent_color,
    v_household.id,
    false
  FROM public.categories d
  WHERE d.is_default = true AND d.household_id IS NULL;

  RETURN v_household;
END;
$$;

REVOKE ALL ON FUNCTION public.create_household(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.create_household(text) TO authenticated;

-- 4) Tighten categories SELECT policy so clients only see household categories
DROP POLICY IF EXISTS "Users can view categories in their households or default categories" ON public.categories;

CREATE POLICY "Users can view categories in their households"
  ON public.categories FOR SELECT
  USING (
    household_id IS NOT NULL
    AND EXISTS (
      SELECT 1 FROM public.household_members
      WHERE household_members.household_id = categories.household_id
      AND household_members.user_id = auth.uid()
    )
  );


