import { useMemo } from 'react'
import { SkillIconImage } from '../../components/SkillIconImage'
import { getItemImage, resolveSkillIcon, skills } from '../../data'
import { useBuild } from '../../store/build'
import { mercAuraKey, mercGrantedAuras } from '../../utils/build/mercStats'
import { CountBadge, Panel } from './primitives'

function SectionLabel({ children }: { children: string }) {
  return (
    <div className="mb-1.5 mt-3 flex items-center gap-2 font-mono text-[9px] uppercase tracking-[0.18em] text-faint first:mt-0">
      <span className="text-accent-hot/80">{children}</span>
      <span aria-hidden className="h-px flex-1 bg-border" />
    </div>
  )
}

export default function ActiveAuraPanel() {
  const classId = useBuild((s) => s.classId)
  const skillRanks = useBuild((s) => s.skillRanks)
  const activeAuraId = useBuild((s) => s.activeAuraId)
  const setActiveAura = useBuild((s) => s.setActiveAura)
  const mercInventory = useBuild((s) => s.mercInventory)
  const mercDisabledAuras = useBuild((s) => s.mercDisabledAuras)
  const setMercAuraDisabled = useBuild((s) => s.setMercAuraDisabled)

  const auraSkills = useMemo(() => {
    if (!classId) return []
    return skills.filter((s) => s.classId === classId && s.kind === 'aura')
  }, [classId])

  const mercAuras = useMemo(
    () => mercGrantedAuras(mercInventory),
    [mercInventory],
  )
  const enabledMercAuras = mercAuras.filter(
    (a) => !mercDisabledAuras[mercAuraKey(a.name)],
  )

  return (
    <Panel
      title="Active Aura"
      subtitle="Select the single aura you are running."
      trailing={
        <CountBadge
          value={(activeAuraId ? 1 : 0) + enabledMercAuras.length}
          total={auraSkills.length + mercAuras.length}
          highlight={!!activeAuraId || enabledMercAuras.length > 0}
        />
      }
    >
      {mercAuras.length > 0 && <SectionLabel>Hero Skills</SectionLabel>}
      {auraSkills.length === 0 ? (
        <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
          No auras available for this class.
        </p>
      ) : (
        <ul className="space-y-2">
          <li>
            <label
              className={`flex cursor-pointer items-center gap-3 rounded-[3px] border px-3 py-2 transition-colors ${
                activeAuraId === null
                  ? 'border-accent-deep'
                  : 'border-border-2 hover:border-accent-deep'
              }`}
              style={{
                background:
                  activeAuraId === null
                    ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                    : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
              }}
            >
              <input
                type="radio"
                name="active-aura"
                checked={activeAuraId === null}
                onChange={() => setActiveAura(null)}
              />
              <span className="text-sm font-medium text-muted">None</span>
            </label>
          </li>
          {auraSkills.map((s) => {
            const rank = skillRanks[s.id] ?? 0
            const ready = rank > 0
            const checked = activeAuraId === s.id
            return (
              <li key={s.id}>
                <label
                  className={`flex items-center justify-between gap-3 rounded-[3px] border px-3 py-2 transition-colors ${
                    ready
                      ? checked
                        ? 'cursor-pointer border-accent-deep'
                        : 'cursor-pointer border-border-2 hover:border-accent-deep'
                      : 'border-border opacity-60'
                  }`}
                  style={{
                    background: checked
                      ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                      : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                    boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
                  }}
                >
                  <span className="flex min-w-0 items-center gap-2">
                    <input
                      type="radio"
                      name="active-aura"
                      checked={checked}
                      onChange={() => setActiveAura(s.id)}
                      disabled={!ready}
                    />
                    <SkillIconImage
                      icon={resolveSkillIcon(s)}
                      size={32}
                      className="text-2xl"
                    />
                    <span className="min-w-0">
                      <div
                        className={`truncate text-sm font-medium ${checked ? 'text-accent-hot' : 'text-text'}`}
                      >
                        {s.name}
                      </div>
                    </span>
                  </span>
                  {!ready && (
                    <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                      not learned
                    </span>
                  )}
                </label>
              </li>
            )
          })}
        </ul>
      )}
      {mercAuras.length > 0 && (
        <>
          <SectionLabel>Mercenary</SectionLabel>
          <ul className="space-y-2">
            {mercAuras.map((aura) => {
              const key = mercAuraKey(aura.name)
              const enabled = !mercDisabledAuras[key]
              const sprite = getItemImage(aura.baseId)
              const level =
                aura.levelMin === aura.levelMax
                  ? `Level ${aura.levelMin}`
                  : `Level [${aura.levelMin}-${aura.levelMax}]`
              return (
                <li key={`${key}-${aura.baseId}`}>
                  <label
                    className={`flex cursor-pointer items-center justify-between gap-3 rounded-[3px] border px-3 py-2 transition-colors ${
                      enabled
                        ? 'border-accent-deep'
                        : 'border-border-2 hover:border-accent-deep'
                    }`}
                    style={{
                      background: enabled
                        ? 'linear-gradient(180deg, rgba(58,46,24,0.5), rgba(28,29,36,0.5))'
                        : 'linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 70%, transparent))',
                      boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.4)',
                    }}
                  >
                    <span className="flex min-w-0 items-center gap-2">
                      <input
                        type="checkbox"
                        checked={enabled}
                        onChange={() => setMercAuraDisabled(key, enabled)}
                      />
                      <span
                        className="flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-[3px] border border-border-2"
                        style={{
                          background:
                            'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                        }}
                      >
                        {sprite ? (
                          <img
                            src={sprite}
                            alt=""
                            draggable={false}
                            className="h-full w-full object-contain select-none"
                            style={{ imageRendering: 'pixelated' }}
                          />
                        ) : (
                          <span className="text-sm text-faint">◆</span>
                        )}
                      </span>
                      <span className="min-w-0">
                        <div
                          className={`truncate text-sm font-medium ${enabled ? 'text-accent-hot' : 'text-text'}`}
                        >
                          {aura.name}
                          <span className="ml-2 font-mono text-[10px] uppercase tracking-[0.12em] text-muted">
                            {level}
                          </span>
                        </div>
                        <div className="truncate text-[10px] text-faint">
                          {aura.itemName}
                        </div>
                      </span>
                    </span>
                    {!enabled && (
                      <span className="shrink-0 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                        not counted
                      </span>
                    )}
                  </label>
                </li>
              )
            })}
          </ul>
        </>
      )}
    </Panel>
  )
}
