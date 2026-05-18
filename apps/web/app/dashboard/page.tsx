import { redirect } from 'next/navigation';
import { createClient } from '@/lib/supabase/server';
import { ConnectLocalSession } from '@/app/components/ConnectLocalSession';
import { isStandaloneApp } from '@/lib/standalone';
import DashboardClient from './DashboardClient';

export default async function DashboardPage() {
  const supabase = await createClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) {
    if (isStandaloneApp()) {
      return <ConnectLocalSession />;
    }
    redirect('/auth/login');
  }

  return <DashboardClient />;
}

