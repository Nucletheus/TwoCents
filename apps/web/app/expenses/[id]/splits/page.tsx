import { redirect } from 'next/navigation';
import { createClient } from '@/lib/supabase/server';
import { ConnectLocalSession } from '@/app/components/ConnectLocalSession';
import { isStandaloneApp } from '@/lib/standalone';
import ExpenseSplitsClient from './ExpenseSplitsClient';

export default async function ExpenseSplitsPage({
  params,
}: {
  params: { id: string };
}) {
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

  return <ExpenseSplitsClient expenseId={params.id} />;
}

