-- Ensure households INSERT is allowed (some environments may be missing this policy)

DROP POLICY IF EXISTS "Users can create households" ON public.households;
CREATE POLICY "Users can create households"
  ON public.households FOR INSERT
  WITH CHECK (true);

-- Ensure role has base privileges (RLS still applies)
GRANT INSERT, SELECT, UPDATE, DELETE ON public.households TO authenticated;


