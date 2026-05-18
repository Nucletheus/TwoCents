-- Persist per-account flags so users only need to set them once.
-- - is_income_account: marks an account as an income source (e.g., payroll)
-- - is_skipped: marks an account to be excluded/ignored by relevant workflows

ALTER TABLE public.accounts
  ADD COLUMN IF NOT EXISTS is_income_account BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS is_skipped BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_accounts_household_income_account
  ON public.accounts (household_id, is_income_account);

CREATE INDEX IF NOT EXISTS idx_accounts_household_is_skipped
  ON public.accounts (household_id, is_skipped);


