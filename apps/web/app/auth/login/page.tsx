import { redirect } from 'next/navigation';
import { isStandaloneApp } from '@/lib/standalone';
import LoginFormClient from './LoginFormClient';

export default function LoginPage() {
  if (isStandaloneApp()) {
    redirect('/dashboard');
  }
  return <LoginFormClient />;
}
