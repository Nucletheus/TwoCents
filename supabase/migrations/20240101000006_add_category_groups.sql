-- Add group_name field to categories table
ALTER TABLE public.categories
ADD COLUMN group_name TEXT;

-- Create index for better query performance when grouping
CREATE INDEX idx_categories_group_name ON public.categories(group_name);

-- Update RLS policies to include group_name in SELECT
-- (No changes needed as RLS policies already allow SELECT on all columns)

