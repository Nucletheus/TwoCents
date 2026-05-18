-- Add exclude_from_calculations and is_income_category fields to categories table
ALTER TABLE public.categories
ADD COLUMN IF NOT EXISTS exclude_from_calculations BOOLEAN DEFAULT false NOT NULL,
ADD COLUMN IF NOT EXISTS is_income_category BOOLEAN DEFAULT false NOT NULL;

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_categories_exclude_from_calculations 
  ON public.categories(exclude_from_calculations) 
  WHERE exclude_from_calculations = true;

CREATE INDEX IF NOT EXISTS idx_categories_is_income_category 
  ON public.categories(is_income_category) 
  WHERE is_income_category = true;

-- Mark existing income group categories as income categories
UPDATE public.categories
SET is_income_category = true
WHERE group_name LIKE 'Income%'
  AND is_income_category = false;

