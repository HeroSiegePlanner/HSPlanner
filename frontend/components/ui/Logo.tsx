import { useId } from 'react'

interface LogoProps {
  size?: number
  glow?: boolean
  className?: string
  title?: string
}

export default function Logo({
  size = 32,
  glow = false,
  className,
  title,
}: LogoProps) {
  const baseId = useId()
  const coreId = `${baseId}-core`
  const lineId = `${baseId}-line`
  const glowId = `${baseId}-glow`

  return (
    <svg
      viewBox="0 0 64 64"
      width={size}
      height={size}
      role={title ? 'img' : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      className={className}
    >
      <defs>
        <radialGradient id={coreId} cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#fff0c4" />
          <stop offset="60%" stopColor="#e8d96b" />
          <stop offset="100%" stopColor="#8a6f2a" />
        </radialGradient>
        <linearGradient id={lineId} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#fff0c4" />
          <stop offset="100%" stopColor="#8a6f2a" />
        </linearGradient>
        {glow && (
          <filter id={glowId} x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="1.4" result="b" />
            <feMerge>
              <feMergeNode in="b" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        )}
      </defs>
      <circle
        cx="32"
        cy="32"
        r="29"
        fill="none"
        stroke={`url(#${lineId})`}
        strokeWidth="1.5"
      />
      <circle
        cx="32"
        cy="32"
        r="29"
        fill="none"
        stroke="#c9a55a"
        strokeWidth="0.5"
        strokeDasharray="2 4"
        opacity="0.4"
      />
      <g filter={glow ? `url(#${glowId})` : undefined}>
        <path
          d="M32 32 L32 8  M32 32 L52.8 44 M32 32 L11.2 44"
          stroke={`url(#${lineId})`}
          strokeWidth="2.2"
          strokeLinecap="round"
        />
        <circle
          cx="32"
          cy="8"
          r="4"
          fill="#1a1610"
          stroke="#c9a55a"
          strokeWidth="1.6"
        />
        <circle
          cx="52.8"
          cy="44"
          r="4"
          fill="#1a1610"
          stroke="#c9a55a"
          strokeWidth="1.6"
        />
        <circle
          cx="11.2"
          cy="44"
          r="4"
          fill="#1a1610"
          stroke="#c9a55a"
          strokeWidth="1.6"
        />
        <g transform="translate(32 32) rotate(45)">
          <rect
            x="-9"
            y="-9"
            width="18"
            height="18"
            fill={`url(#${coreId})`}
            stroke="#fff0c4"
            strokeWidth="1"
          />
          <rect
            x="-4"
            y="-4"
            width="8"
            height="8"
            fill="none"
            stroke="#5a4520"
            strokeWidth="0.8"
            transform="rotate(-45)"
          />
        </g>
      </g>
    </svg>
  )
}
