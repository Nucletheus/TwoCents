-- Add sub_description field to expenses table
ALTER TABLE public.expenses 
ADD COLUMN IF NOT EXISTS sub_description TEXT;

