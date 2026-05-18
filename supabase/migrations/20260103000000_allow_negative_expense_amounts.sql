-- Allow negative amounts in expenses table to support CSV imports with negative values
-- This enables easier tracking of expenses vs income/credits

ALTER TABLE public.expenses
DROP CONSTRAINT IF EXISTS expenses_amount_check;

-- Remove the constraint that was preventing negative values
-- The amount column will now accept any decimal value (positive or negative)

