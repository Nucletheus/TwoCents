import { redirect } from 'next/navigation';
import { createClient } from '@/lib/supabase/server';
import { ConnectLocalSession } from '@/app/components/ConnectLocalSession';
import { isStandaloneApp } from '@/lib/standalone';
import AccountsClient from './AccountsClient';

export default async function AccountsPage() {
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

  return <AccountsClient />;
}

