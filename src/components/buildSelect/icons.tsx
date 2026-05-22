// SVG icon set for the Build Select library screen. Replaces the unicode
// glyphs the screen used to render in its toolbar, search field, folder tree
// and preview pane — crisp strokes that match the rest of the app's
// iconography (Logo, KofiIcon, the FolderTree folder/star icons).

import type { ReactNode } from 'react'

interface IconProps {
  className?: string
}

/** Shared 16×16 stroked-icon frame. Colour follows `currentColor`. */
function StrokeIcon({
  className,
  children,
}: {
  className?: string
  children: ReactNode
}) {
  return (
    <svg
      className={className}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      {children}
    </svg>
  )
}

/** Solid play triangle — "Open Build". */
export function PlayIcon({ className }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 16 16" aria-hidden>
      <path fill="currentColor" d="M5 3.4 L12.2 8 L5 12.6 Z" />
    </svg>
  )
}

/** Plus — "New" / "New Folder" affordances. */
export function PlusIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <path d="M8 3.5 V12.5 M3.5 8 H12.5" />
    </StrokeIcon>
  )
}

/** Down-arrow into a tray — "Import". */
export function ImportIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <path d="M8 2.6 V9.6" />
      <path d="M5.2 6.9 L8 9.8 L10.8 6.9" />
      <path d="M3 11 V13 H13 V11" />
    </StrokeIcon>
  )
}

/** Floppy disk — "Save". */
export function SaveIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <path d="M3.2 3.2 H10.4 L12.8 5.6 V12.8 H3.2 Z" />
      <path d="M5.6 3.2 V6.2 H9.6 V3.2" />
      <rect x="5.6" y="8.6" width="4.8" height="4.2" rx="0.5" />
    </StrokeIcon>
  )
}

/** Overlapping sheets — "Copy". */
export function CopyIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <rect x="5.4" y="5.4" width="7.2" height="7.2" rx="1.2" />
      <path d="M3.4 10.4 V4 A0.6 0.6 0 0 1 4 3.4 H10" />
    </StrokeIcon>
  )
}

/** Pencil — "Rename". */
export function RenameIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <path d="M9.8 3.5 L12.5 6.2 L5.6 13.1 H2.9 V10.4 Z" />
      <path d="M8.6 4.7 L11.3 7.4" />
    </StrokeIcon>
  )
}

/** Trash can — "Delete". */
export function DeleteIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <path d="M3.6 5 H12.4" />
      <path d="M6.3 5 V3.6 A0.6 0.6 0 0 1 6.9 3 H9.1 A0.6 0.6 0 0 1 9.7 3.6 V5" />
      <path d="M4.9 5 L5.5 12.6 A0.8 0.8 0 0 0 6.3 13.3 H9.7 A0.8 0.8 0 0 0 10.5 12.6 L11.1 5" />
    </StrokeIcon>
  )
}

/** Folder with a plus — "New Folder". */
export function NewFolderIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <path d="M2.5 5 H6.4 L7.5 6.6 H13.5 V12.5 H2.5 Z" />
      <path d="M8 8.3 V11 M6.7 9.65 H9.3" />
    </StrokeIcon>
  )
}

/** Magnifier — search fields. */
export function SearchIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <circle cx="7" cy="7" r="3.8" />
      <path d="M9.8 9.8 L13.3 13.3" />
    </StrokeIcon>
  )
}

/** Right-pointing chevron — folder twist + folder-choice marker. */
export function CaretIcon({ className }: IconProps) {
  return (
    <StrokeIcon className={className}>
      <path d="M6 3.8 L10.2 8 L6 12.2" />
    </StrokeIcon>
  )
}
