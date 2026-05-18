import { createServerClient } from '@supabase/ssr';
import { NextResponse, type NextRequest } from 'next/server';
import { getSupabaseEnv } from './lib/env';
import { isStandaloneApp } from './lib/standalone';

export async function middleware(request: NextRequest) {
  if (isStandaloneApp()) {
    const path = request.nextUrl.pathname;
    if (path === '/' || path.startsWith('/auth')) {
      if (path.startsWith('/auth/logout') && request.method !== 'GET') {
        // allow POST so signOut can run if something still calls it
      } else if (path.startsWith('/auth/logout') && request.method === 'GET') {
        return NextResponse.redirect(new URL('/dashboard', request.url));
      } else {
        return NextResponse.redirect(new URL('/dashboard', request.url));
      }
    }
  }

  const supabaseResponse = NextResponse.next({ request });
  const { url, anonKey } = getSupabaseEnv();

  const supabase = createServerClient(
    url,
    anonKey,
    {
      cookies: {
        // @supabase/ssr@0.1.0 expects cookies.get/set/remove
        get(name: string) {
          return request.cookies.get(name)?.value;
        },
        set(name: string, value: string, options: any) {
          supabaseResponse.cookies.set(name, value, options);
        },
        remove(name: string, options: any) {
          supabaseResponse.cookies.set(name, '', { ...options, maxAge: 0 });
        },
      },
    }
  );

  // Refresh session if expired - required for Server Components
  await supabase.auth.getUser();

  return supabaseResponse;
}

export const config = {
  matcher: [
    /*
     * Match all request paths except for the ones starting with:
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     * Feel free to modify this pattern to include more paths.
     */
    '/((?!_next/static|_next/image|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)',
  ],
};

