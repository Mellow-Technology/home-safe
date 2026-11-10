/**
 * The Home Safe mark: a white house with a heart inside, on the app icon's
 * teal tile. Same shapes as scripts/make-icons.py.
 */
export function Logo({ size = 30 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden focusable="false">
      <defs>
        <linearGradient id="logo-tile" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#149187" />
          <stop offset="1" stopColor="#0c625c" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="7.2" fill="url(#logo-tile)" />
      <path d="M20.2 7.7h2.2v6.4h-2.2zM16 5.8 4.9 15.1h3.7v10.5h14.8V15.1h3.7z" fill="#fff" />
      <path d="M16 23.4c-4.6-3-5.6-5.4-4.4-6.9 1-1.2 3-1 4.4.8 1.4-1.8 3.4-2 4.4-.8 1.2 1.5.2 3.9-4.4 6.9z" fill="#f43f5e" />
    </svg>
  );
}

/** A heart, in the current colour. */
export function Heart({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" aria-hidden focusable="false" fill="currentColor">
      <path d="M8 14.2C2.4 10.6.8 7.6 2.3 5.4 3.6 3.6 6.4 3.7 8 6c1.6-2.3 4.4-2.4 5.7-.6 1.5 2.2-.1 5.2-5.7 8.8z" />
    </svg>
  );
}
