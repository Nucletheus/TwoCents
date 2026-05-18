-- Create analytics_insights table for storing AI-generated insights per timeframe
CREATE TABLE IF NOT EXISTS public.analytics_insights (
  id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
  household_id UUID NOT NULL REFERENCES public.households(id) ON DELETE CASCADE,
  timeframe_type TEXT NOT NULL CHECK (timeframe_type IN ('month', 'year', 'all', 'custom')),
  timeframe_key TEXT NOT NULL, -- e.g., '2024-01' for month, '2024' for year, 'all' for all, or date range for custom
  insights_text TEXT NOT NULL,
  generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  created_by UUID REFERENCES auth.users(id) ON DELETE SET NULL,
  UNIQUE(household_id, timeframe_type, timeframe_key)
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_analytics_insights_household_id ON public.analytics_insights(household_id);
CREATE INDEX IF NOT EXISTS idx_analytics_insights_timeframe ON public.analytics_insights(household_id, timeframe_type, timeframe_key);

-- Enable Row Level Security
ALTER TABLE public.analytics_insights ENABLE ROW LEVEL SECURITY;

-- RLS Policies
CREATE POLICY "Users can view analytics insights in their households"
  ON public.analytics_insights FOR SELECT
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = analytics_insights.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can create analytics insights in their households"
  ON public.analytics_insights FOR INSERT
  WITH CHECK (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = analytics_insights.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can update analytics insights in their households"
  ON public.analytics_insights FOR UPDATE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = analytics_insights.household_id
      AND hm.user_id = auth.uid()
    )
  );

CREATE POLICY "Users can delete analytics insights in their households"
  ON public.analytics_insights FOR DELETE
  USING (
    EXISTS (
      SELECT 1 FROM public.household_members hm
      WHERE hm.household_id = analytics_insights.household_id
      AND hm.user_id = auth.uid()
    )
  );

