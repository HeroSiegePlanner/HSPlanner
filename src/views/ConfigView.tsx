import CharacterBasics from './config/CharacterBasics'
import { GroupHeading } from './config/primitives'
import ActiveBuffsPanel from './config/ActiveBuffsPanel'
import ActiveAuraPanel from './config/ActiveAuraPanel'
import ProcsPanel from './config/ProcsPanel'
import EnemyConditionsPanel from './config/EnemyConditionsPanel'
import PlayerConditionsPanel from './config/PlayerConditionsPanel'
import ResistancesPanel from './config/ResistancesPanel'
import SkillProjectilesPanel from './config/SkillProjectilesPanel'
import CustomStatsPanel from './config/CustomStatsPanel'

export default function ConfigView() {
  return (
    <div className="space-y-8">
      <header>
        <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 8px rgba(224,184,100,0.6)' }}
          />
          Setup · character & encounter
        </div>
        <h2
          className="m-0 text-[22px] font-semibold tracking-[0.02em] text-accent-hot"
          style={{ textShadow: '0 0 16px rgba(224,184,100,0.18)' }}
        >
          Configuration
        </h2>
      </header>

      <section className="space-y-4">
        <GroupHeading
          title="Character"
          subtitle="Class, level and attribute allocation."
        />
        <CharacterBasics />
      </section>

      <section className="space-y-4">
        <GroupHeading
          title="Encounter & Combat"
          subtitle="Buffs, procs, enemy and player state, and manual overrides the calculator reads."
        />

        <ActiveBuffsPanel />
        <ActiveAuraPanel />
        <ProcsPanel />
        <EnemyConditionsPanel />
        <PlayerConditionsPanel />
        <ResistancesPanel />
        <SkillProjectilesPanel />
        <CustomStatsPanel />
      </section>
    </div>
  )
}
