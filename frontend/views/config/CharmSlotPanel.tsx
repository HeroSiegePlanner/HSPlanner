import { Panel } from './configPrimitives'
import { useSettings } from '../../store/settings'

export default function CharmSlotPanel() {
  const extraCharmSlot = useSettings((s) => s.extraCharmSlot)
  const setExtraCharmSlot = useSettings((s) => s.setExtraCharmSlot)

  return (
    <Panel
      title="Charm Inventory"
      subtitle="Whether your character has unlocked the extra charm cell in-game."
    >
      <label className="flex cursor-pointer flex-col gap-1">
        <span className="flex items-center gap-2.5">
          <input
            type="checkbox"
            checked={extraCharmSlot}
            onChange={(e) => setExtraCharmSlot(e.target.checked)}
            className="shrink-0"
          />
          <span className="text-[13px] font-semibold text-text">
            Extra charm slot unlocked
          </span>
        </span>
        <span className="pl-6 text-[12px] leading-snug text-muted">
          Adds the unlockable 30th cell to the charm grid in the Gear tab.
          Stored on this device, shared by all builds.
        </span>
      </label>
    </Panel>
  )
}
