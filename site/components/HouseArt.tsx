/**
 * A little illustrated house, standing in for a listing photo.
 */
export interface HouseArtProps {
  sky: [string, string];
  wall: string;
  roof: string;
  door: string;
  kind: "gable" | "cottage" | "modern" | "tall";
  /** The sun, or the moon and a lit window for an evening scene */
  evening?: boolean;
}

export function HouseArt({ sky, wall, roof, door, kind, evening }: HouseArtProps) {
  const id = `sky-${sky.join("").replace(/#/g, "")}`;
  const glow = evening ? "#ffd27a" : "#e9f3f7";
  return (
    <svg viewBox="0 0 160 100" preserveAspectRatio="xMidYMid slice" aria-hidden focusable="false">
      <defs>
        <linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor={sky[0]} />
          <stop offset="1" stopColor={sky[1]} />
        </linearGradient>
      </defs>
      <rect width="160" height="100" fill={`url(#${id})`} />
      <circle cx="128" cy="22" r="9" fill={evening ? "#fff4d6" : "#ffd98a"} opacity={evening ? 0.9 : 1} />
      <ellipse cx="34" cy="24" rx="16" ry="5" fill="#fff" opacity="0.55" />
      <rect y="78" width="160" height="22" fill="#8fbf7a" />
      <rect y="78" width="160" height="3" fill="#7aae67" />
      {/* Trees */}
      <circle cx="18" cy="64" r="12" fill="#5f9e63" />
      <rect x="16.5" y="68" width="3" height="11" fill="#7a5a43" />
      <circle cx="146" cy="68" r="9" fill="#6aa96b" />
      <rect x="144.8" y="70" width="2.4" height="9" fill="#7a5a43" />

      {kind === "gable" && (
        <g>
          <rect x="50" y="48" width="60" height="31" fill={wall} />
          <path d="M44 50 80 24l36 26z" fill={roof} />
          <rect x="74" y="60" width="12" height="19" rx="1.5" fill={door} />
          <rect x="56" y="56" width="12" height="10" rx="1" fill={glow} />
          <rect x="92" y="56" width="12" height="10" rx="1" fill={glow} />
          <path d="M80 37.5c-3-2-3.6-3.6-2.8-4.6.7-.8 2-.6 2.8.5.8-1.1 2.1-1.3 2.8-.5.8 1-.2 2.6-2.8 4.6z" fill="#f43f5e" />
        </g>
      )}
      {kind === "cottage" && (
        <g>
          <rect x="54" y="52" width="54" height="27" rx="2" fill={wall} />
          <path d="M48 54c10-18 22-26 33-26s23 8 33 26z" fill={roof} />
          <rect x="94" y="30" width="7" height="14" fill={roof} />
          <path d="M75 79V66a6 6 0 0 1 12 0v13z" fill={door} />
          <circle cx="64" cy="64" r="5" fill={glow} />
          <circle cx="98" cy="64" r="5" fill={glow} />
        </g>
      )}
      {kind === "modern" && (
        <g>
          <rect x="42" y="44" width="50" height="35" fill={wall} />
          <rect x="88" y="34" width="34" height="45" fill={roof} />
          <rect x="38" y="40" width="58" height="5" fill={door} />
          <rect x="48" y="52" width="36" height="14" fill={glow} opacity="0.95" />
          <rect x="96" y="42" width="18" height="16" fill={glow} />
          <rect x="99" y="64" width="11" height="15" fill={door} />
        </g>
      )}
      {kind === "tall" && (
        <g>
          <rect x="58" y="36" width="46" height="43" fill={wall} />
          <path d="M53 38 81 18l28 20z" fill={roof} />
          <rect x="64" y="44" width="10" height="11" rx="1" fill={glow} />
          <rect x="88" y="44" width="10" height="11" rx="1" fill={glow} />
          <rect x="64" y="61" width="10" height="11" rx="1" fill={glow} />
          <rect x="85" y="61" width="12" height="18" rx="1.5" fill={door} />
          <rect x="81" y="74" width="20" height="5" fill="#d8c8b8" />
        </g>
      )}
    </svg>
  );
}
