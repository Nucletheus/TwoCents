import { redirect } from 'next/navigation';
import { isStandaloneApp } from '@/lib/standalone';
import SignUpFormClient from './SignUpFormClient';

export default function SignUpPage() {
  if (isStandaloneApp()) {
    redirect('/dashboard');
  }
  return <SignUpFormClient />;
}
