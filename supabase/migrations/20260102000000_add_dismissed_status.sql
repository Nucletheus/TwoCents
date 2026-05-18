-- Add 'dismissed' status to expenses table
ALTER TABLE public.expenses
DROP CONSTRAINT IF EXISTS expenses_status_check;

ALTER TABLE public.expenses
ADD CONSTRAINT expenses_status_check 
CHECK (status IN ('pending_review', 'approved', 'flagged', 'dismissed'));

