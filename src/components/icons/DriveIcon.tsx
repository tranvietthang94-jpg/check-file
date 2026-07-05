interface DriveIconProps {
  removable?: boolean;
  className?: string;
}

/**
 * Small hand-drawn outline icons distinguishing removable media (memory
 * card shape) from fixed/internal drives (rectangle + activity dot) --
 * deliberately not a copy of any third-party icon set.
 */
export function DriveIcon({ removable, className }: DriveIconProps) {
  if (removable) {
    return (
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        className={className}
        aria-hidden="true"
      >
        <path d="M7.5 3.5h6l4 4v12a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1v-15a1 1 0 0 1 1-1Z" />
        <path d="M13.5 3.5v4h4" />
        <path d="M9.5 13h5" />
        <path d="M9.5 16.5h5" />
      </svg>
    );
  }

  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      <rect x="3" y="6" width="18" height="12" rx="2.5" />
      <circle cx="7.5" cy="12" r="1" fill="currentColor" stroke="none" />
      <path d="M11.5 12h6.5" />
    </svg>
  );
}
