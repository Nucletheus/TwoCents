/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  transpilePackages: ['@twocents/shared', '@twocents/supabase'],
  experimental: {
    externalDir: true,
  },
};

module.exports = nextConfig;

