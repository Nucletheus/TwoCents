/**
 * Local standalone build: no sign-in UI; one automatic session (see STANDALONE_AUTH_* in .env).
 * Default is standalone (on). Set NEXT_PUBLIC_STANDALONE_MODE=false to enable /auth login and sign-up
 * (e.g. a hosted or multi-tenant deploy).
 */
export function isStandaloneApp(): boolean {
  return process.env.NEXT_PUBLIC_STANDALONE_MODE !== 'false';
}
