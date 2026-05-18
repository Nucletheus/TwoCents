'use client';

import { useTheme } from './ThemeProvider';

export default function Logo({ className = 'h-8 w-8' }: { className?: string }) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  return (
    <svg
      className={className}
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-label="TwoCents Logo"
    >
      {/* Two overlapping coins forming a "2" shape */}
      {/* First coin (left, slightly tilted) */}
      <circle
        cx="12"
        cy="16"
        r="8"
        className="fill-palette-primary"
        style={{ fill: 'var(--palette-primary)' }}
        opacity="0.9"
        transform="rotate(-5 12 16)"
      />
      {/* Second coin (right, overlapping) */}
      <circle
        cx="20"
        cy="16"
        r="8"
        className="fill-palette-secondary"
        style={{ fill: 'var(--palette-secondary)', mixBlendMode: 'multiply' }}
        opacity="0.9"
        transform="rotate(5 20 16)"
      />
    </svg>
  );
}
