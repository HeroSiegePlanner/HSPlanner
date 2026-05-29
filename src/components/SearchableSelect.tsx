import { useRef, useState } from 'react'
import type { ReactNode } from 'react'
import HoverPortal from './HoverPortal'
import Dropdown, { type DropdownOption } from './Dropdown'

export interface SearchableOption {
  id: string
  label: string
  hint?: string
  accent?: string
}

interface Props {
  value: string | null
  options: SearchableOption[]
  placeholder?: string
  onChange: (id: string | null) => void
  emptyLabel?: string
  clearLabel?: string
  sidePanel?: (hoveredId: string | null) => ReactNode
}

// Thin wrapper over the canonical Dropdown; maps option hints to its meta line.
export default function SearchableSelect({
  value,
  options,
  placeholder = 'Select…',
  onChange,
  emptyLabel = 'No results',
  clearLabel = '— none —',
  sidePanel,
}: Props) {
  const [hoveredId, setHoveredId] = useState<string | null>(null)
  const [open, setOpen] = useState(false)
  const boxRef = useRef<HTMLDivElement>(null)

  const ddOptions: DropdownOption[] = options.map((o) => ({
    id: o.id,
    label: o.label,
    meta: o.hint,
  }))

  return (
    <div className="relative" ref={boxRef}>
      <Dropdown
        value={value}
        options={ddOptions}
        onChange={onChange}
        placeholder={placeholder}
        emptyLabel={emptyLabel}
        clearLabel={clearLabel}
        onHoverChange={setHoveredId}
        onOpenChange={(o) => {
          setOpen(o)
          if (!o) setHoveredId(null)
        }}
      />
      {sidePanel && open && (
        <HoverPortal anchorRef={boxRef}>{sidePanel(hoveredId)}</HoverPortal>
      )}
    </div>
  )
}
