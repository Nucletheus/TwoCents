import Link from 'next/link';
import { redirect } from 'next/navigation';
import { isStandaloneApp } from '@/lib/standalone';

export default function Home() {
  if (isStandaloneApp()) {
    redirect('/dashboard');
  }
  return (
    <main className="flex min-h-screen flex-col items-center justify-center p-24 bg-gradient-to-b from-indigo-50 to-white">
      <div className="z-10 max-w-5xl w-full items-center justify-center text-center">
        <h1 className="text-6xl font-bold mb-4 text-gray-900">
          TwoCents
        </h1>
        <p className="text-xl text-gray-600 mb-12">
          Finance and budgeting app for couples
        </p>
        <div className="flex gap-4 justify-center">
          <Link
            href="/auth/signup"
            className="rounded-md bg-indigo-600 px-8 py-3 text-lg font-medium text-white hover:bg-indigo-700 transition-colors"
          >
            Get Started
          </Link>
          <Link
            href="/auth/login"
            className="rounded-md border border-indigo-600 px-8 py-3 text-lg font-medium text-indigo-600 hover:bg-indigo-50 transition-colors"
          >
            Sign In
          </Link>
        </div>
      </div>
    </main>
  );
}

