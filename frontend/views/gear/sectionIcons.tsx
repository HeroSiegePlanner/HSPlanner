export type SectionIconKind =
  | 'sockets'
  | 'rolls'
  | 'runeword'
  | 'stars'
  | 'randomSkill'
  | 'element'
  | 'affixes'
  | 'forged'
  | 'augment'
  | 'set'

const PATHS: Record<SectionIconKind, React.ReactNode> = {
  sockets: (
    <>
      <circle cx="6" cy="6" r="4.2" />
      <circle cx="6" cy="6" r="1.4" fill="currentColor" stroke="none" />
    </>
  ),
  rolls: (
    <>
      <path d="M1 6h10" />
      <circle cx="7.5" cy="6" r="1.9" fill="currentColor" stroke="none" />
    </>
  ),
  runeword: <path d="M4 1v10M4 1l4 2.5L4 6l4 5" />,
  stars: (
    <path
      d="M6 1l1.45 3.1 3.3.4-2.45 2.3.65 3.3L6 8.6 3.05 10.1l.65-3.3L1.25 4.5l3.3-.4z"
      fill="currentColor"
      stroke="none"
    />
  ),
  randomSkill: (
    <>
      <rect x="1.5" y="1.5" width="9" height="9" rx="1.6" />
      <circle cx="4" cy="4" r="0.9" fill="currentColor" stroke="none" />
      <circle cx="6" cy="6" r="0.9" fill="currentColor" stroke="none" />
      <circle cx="8" cy="8" r="0.9" fill="currentColor" stroke="none" />
    </>
  ),
  element: (
    <path d="M6 1.2c1.8 2 3.4 3.7 3.4 5.6a3.4 3.4 0 1 1-6.8 0C2.6 4.9 4.2 3.2 6 1.2z" />
  ),
  affixes: (
    <>
      <path d="M1.5 2.5h4l5 5-3.5 3.5-5-5v-3.5z" />
      <circle cx="3.6" cy="4.6" r="0.8" fill="currentColor" stroke="none" />
    </>
  ),
  forged: (
    // Inverted pentagram — matches the Satanic Crystal name.
    <>
      <circle cx="6" cy="6" r="5" />
      <path d="M6 11L3.1 2l7.7 5.6H1.2L8.9 2z" strokeWidth="1" />
    </>
  ),
  augment: (
    <path
      d="M6 1l1.2 3.8L11 6 7.2 7.2 6 11 4.8 7.2 1 6l3.8-1.2z"
      fill="currentColor"
      stroke="none"
    />
  ),
  set: (
    <>
      <rect x="1.5" y="4" width="6.5" height="6.5" rx="1" />
      <path d="M4 4V2.5A1 1 0 0 1 5 1.5h4.5a1 1 0 0 1 1 1V7a1 1 0 0 1-1 1H8" />
    </>
  ),
}

export function SectionIcon({ kind }: { kind: SectionIconKind }) {
  return (
    <svg
      viewBox="0 0 12 12"
      width="13"
      height="13"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {PATHS[kind]}
    </svg>
  )
}
