import type { LootFilter } from '../../types'
import { Panel } from '../../components/ui/Panel'
import { RARITY_LABEL, RARITY_TEXT } from '../../utils/item/rarity'
import {
  FILTER_RARITIES,
  ITEM_TYPES,
  ITEM_TYPE_LABELS,
  SOCKET_COUNT,
  TIER_LABELS,
  WEAPON_TYPES,
} from '../../utils/lootfilter/constants'
import type { ApplyFilter } from './FilterEditor'
import {
  CellLegend,
  FILTER_BTN_CLASS,
  FILTER_CHIP_CLASS,
  FilterCell,
} from './FilterCells'
import {
  copyTypeConfig,
  isWeaponTypeEnabled,
  rarityCellState,
  socketCellState,
  toggleRarityHighlight,
  toggleRarityRow,
  toggleRarityTier,
  toggleRarityVisible,
  toggleSocketHighlight,
  toggleSocketVisible,
  toggleWeaponType,
} from './filterModel'

const HEADER_BTN_CLASS =
  'h-[16px] w-[16px] rounded-[2px] font-mono text-[9px] font-semibold uppercase text-accent-deep transition-colors hover:text-accent-hot'

const SUBLABEL_CLASS =
  'mb-2 font-mono text-[9.5px] uppercase tracking-[0.14em] text-faint'

interface TypePanelProps {
  filter: LootFilter
  typeId: number
  apply: ApplyFilter
}

export function TypePanel({ filter, typeId, apply }: TypePanelProps) {
  const type = filter.types[typeId]
  const label = ITEM_TYPE_LABELS.get(typeId) ?? `t${typeId}`
  if (!type) return null

  const isWeapon = typeId === 3

  return (
    <Panel
      title={`${label} · visibility`}
      trailing={<CopyControls filter={filter} typeId={typeId} apply={apply} />}
    >
      <div className="flex flex-wrap items-start gap-x-10 gap-y-5">
        <div>
          <div className={SUBLABEL_CLASS}>Rarity × tier</div>
          <table className="border-separate border-spacing-[3px]">
            <thead>
              <tr>
                <th aria-hidden className="w-[74px]" />
                {TIER_LABELS.map((t, tier) => (
                  <th key={t} className="pb-0.5">
                    <button
                      type="button"
                      title={`Toggle tier ${t} for every rarity`}
                      onClick={() => apply(toggleRarityTier(filter, typeId, tier))}
                      className={HEADER_BTN_CLASS}
                    >
                      {t}
                    </button>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {FILTER_RARITIES.map((rarity, r) => (
                <tr key={rarity}>
                  <td className="pr-2">
                    <button
                      type="button"
                      title={`Toggle ${RARITY_LABEL[rarity]} on every tier`}
                      onClick={() => apply(toggleRarityRow(filter, typeId, r))}
                      className={`font-mono text-[11px] transition-opacity hover:opacity-70 ${RARITY_TEXT[rarity]}`}
                    >
                      {RARITY_LABEL[rarity]}
                    </button>
                  </td>
                  {TIER_LABELS.map((t, tier) => (
                    <td key={t}>
                      <FilterCell
                        state={rarityCellState(type.tiers[tier]!.rs, r)}
                        title={`${label} · ${RARITY_LABEL[rarity]} · tier ${t}`}
                        onToggle={() =>
                          apply(toggleRarityVisible(filter, typeId, tier, r))
                        }
                        onHighlight={() =>
                          apply(toggleRarityHighlight(filter, typeId, tier, r))
                        }
                      />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div>
          <div className={SUBLABEL_CLASS}>Socket count</div>
          <table className="border-separate border-spacing-[3px]">
            <thead>
              <tr>
                {Array.from({ length: SOCKET_COUNT }, (_, s) => (
                  <th key={s} className="pb-0.5">
                    <span className="inline-flex h-[16px] w-[16px] items-center justify-center font-mono text-[9px] font-semibold text-accent-deep">
                      {s + 1}
                    </span>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              <tr>
                {Array.from({ length: SOCKET_COUNT }, (_, s) => (
                  <td key={s}>
                    <FilterCell
                      state={socketCellState(type, s)}
                      title={`${label} · ${s + 1} socket${s > 0 ? 's' : ''}`}
                      onToggle={() => apply(toggleSocketVisible(filter, typeId, s))}
                      onHighlight={() =>
                        apply(toggleSocketHighlight(filter, typeId, s))
                      }
                    />
                  </td>
                ))}
              </tr>
            </tbody>
          </table>
        </div>

        {isWeapon && (
          <div className="min-w-0 flex-1">
            <div className={SUBLABEL_CLASS}>Weapon types</div>
            <div className="grid grid-cols-4 gap-1.5">
              {WEAPON_TYPES.map((w, bit) => {
                if (!w) return null
                const enabled = isWeaponTypeEnabled(filter, bit)
                return (
                  <button
                    key={w}
                    type="button"
                    onClick={() => apply(toggleWeaponType(filter, bit))}
                    className={`${FILTER_CHIP_CLASS} tracking-[0.04em] normal-case ${
                      enabled
                        ? 'border-accent-deep text-accent-hot'
                        : 'border-border-2 text-faint hover:text-muted'
                    }`}
                  >
                    {w}
                  </button>
                )
              })}
            </div>
          </div>
        )}
      </div>

      <div className="mt-4 border-t border-border pt-3">
        <CellLegend />
      </div>
    </Panel>
  )
}

function CopyControls({ filter, typeId, apply }: TypePanelProps) {
  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        title="Copy this type's whole configuration to every other type"
        onClick={() =>
          apply(
            copyTypeConfig(
              filter,
              typeId,
              ITEM_TYPES.map((t) => t.id),
            ),
          )
        }
        className={FILTER_BTN_CLASS}
      >
        Copy to all
      </button>
      <select
        value=""
        onChange={(e) => {
          const next = Number(e.target.value)
          if (e.target.value !== '' && !Number.isNaN(next)) {
            apply(copyTypeConfig(filter, typeId, [next]))
          }
        }}
        aria-label="Copy configuration to a specific type"
        className="h-[28px] rounded-[3px] border border-border-2 bg-panel-2 px-2 text-[12px] text-muted outline-none transition-colors hover:border-accent-deep focus:border-accent-deep"
      >
        <option value="">Copy to…</option>
        {ITEM_TYPES.filter((t) => t.id !== typeId).map((t) => (
          <option key={t.id} value={t.id}>
            {t.label}
          </option>
        ))}
      </select>
    </div>
  )
}
