import { redirect } from 'next/navigation';
import { isStandaloneApp } from '@/lib/standalone';
import InvitePageClient from './InvitePageClient';

export default function InvitePage({ params }: { params: Promise<{ token: string }> }) {
  if (isStandaloneApp()) {
    redirect('/households');
  }
  return <InvitePageClient params={params} />;
}
