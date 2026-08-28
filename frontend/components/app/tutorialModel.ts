import type { Section } from '../../App'
import { SEASONS } from '@data/seasons/registry'

export const TUTORIAL_DONE_KEY = 'hsplanner.tutorial.done.v1'

export interface TutorialStep {
  target?: string
  section?: Section
  // CSS selector clicked on step entry (opens modals/overlays for the step)...
  act?: string
  // ...and one clicked on step exit to close what act opened
  undo?: string
  title: string
  body: string
}

export const TUTORIAL_STEPS: TutorialStep[] = [
  {
    title: 'Le Fish au Tutorial',
    body: '*pulp* A guided tour through every part of the planner. Use Next or the arrow keys to move around, and Esc to skip at any time.',
  },
  {
    target: 'sections',
    title: 'Sections',
    body: 'Everything lives in these tabs. The tour walks through each one, then the tools around them.',
  },
  {
    section: 'character',
    target: 'view',
    title: 'Character',
    body: 'A dashboard of the whole build (quick summary): total DPS, resistances and defense, active skills, buffs and procs.',
  },
  {
    section: 'tree',
    target: 'view',
    title: 'Talent Tree',
    body: 'Left-click a node to add points, right-click to remove them. Drag to pan, scroll to zoom — and Ctrl+Z / Ctrl+Y undo and redo any change.',
  },
  {
    section: 'tree',
    target: 'tree-search',
    title: 'Tree Search',
    body: 'Find nodes by name or internal #id — Ctrl+F focuses it from anywhere in this tab. Fit and Reset sit next to it, and Suggest recommends nodes to take next.',
  },
  {
    section: 'tree',
    target: 'suggest-modal',
    act: '[data-tour="tree-suggest"]',
    undo: '[data-tour="suggest-modal"] [aria-label="Close"]',
    title: 'Suggest',
    body: 'Set a point budget between 1 to 200 and let the engine propose valuable incarnation nodes. Suggestions preview on the tree before you apply them.',
  },
  {
    section: 'ether',
    target: 'view',
    title: 'Ether Realm',
    body: 'A second tree with its own point pool. Same controls as the talent tree, including Ctrl+F search.',
  },
  {
    section: 'skills',
    target: 'view',
    title: 'Skills',
    body: 'The skill tree of your class: allocate ranks, expand subskills and mark the skills your build actively uses — damage numbers flow from here.',
  },
  {
    section: 'skills',
    target: 'subtree-button',
    title: 'Subskill Button',
    body: 'Skills that have a subtree show this gear badge on their icon — not every skill does. Click it to open the subskill tree.',
  },
  {
    section: 'skills',
    target: 'subtree-overlay',
    act: '[data-tour="subtree-button"]',
    undo: '[data-tour="subtree-overlay"] [aria-label="Close"]',
    title: 'Subskill Tree',
    body: 'Subskill picks flow into the damage math.',
  },
  {
    section: 'gear',
    target: 'view',
    title: 'Gear',
    body: 'Your equipment, laid out like in game — plus charms and per-build item storage.',
  },
  {
    section: 'gear',
    target: 'gear-doll',
    title: 'Equipment',
    body: 'Click any slot to pick or edit an item. Hovering shows full tooltips with stat comparisons.',
  },
  {
    section: 'gear',
    target: 'gear-slot-modal',
    act: '[data-tour="slot-weapon"]',
    undo: '[data-tour="gear-slot-modal"] [aria-label="Close"]',
    title: 'Item Picker',
    body: 'Each slot opens a picker: choose a base and rarity, then configure rolls, sockets and stars. Edit Text lets you edit the item as raw text.',
  },
  {
    section: 'gear',
    target: 'gear-stash',
    title: 'Stash',
    body: "Per-build storage for spare items — park pieces you're comparing without losing their rolls.",
  },
  {
    section: 'gear',
    target: 'gear-upgrades',
    title: 'Upgrade Advisor',
    body: 'Compares your item bases against the best base for each slot using engine DPS, and points at the slot with the most to gain.',
  },
  {
    section: 'merc',
    target: 'view',
    title: 'Mercenary',
    body: "Mercenary loadout and skills. Merc gear and auras feed back into your hero — including Magic Find, which counts toward your hero's total.",
  },
  {
    section: 'stats',
    target: 'view',
    title: 'Stats',
    body: 'Every number the calculator produces: attributes, offense, defense, EHP and per-skill damage breakdowns.',
  },
  {
    section: 'stats',
    target: 'stats-search',
    title: 'Stat Search',
    body: 'Search covers stats, attributes and skills — Ctrl+F works here too. The chips next to it filter by category.',
  },
  {
    section: 'config',
    target: 'view',
    title: 'Config',
    body: 'Class, level and attribute allocation live here, plus encounter and combat settings: buffs, procs, enemy state and manual overrides the calculator reads.',
  },
  {
    section: 'notes',
    target: 'view',
    title: 'Notes',
    body: 'Freeform notes saved with the build — rotations, shopping lists, reminders, guides, etc.',
  },
  {
    section: 'filters',
    target: 'view',
    title: 'Loot Filters',
    body: 'Build a loot filter from the affixes your build wears and export it as a game-ready filter string. It is looks like loot filter V2 :D',
  },
  {
    target: 'left-stats',
    title: 'Live Stats',
    body: 'This panel is always in view and recalculates with every point you spend, item you equip and toggle you flip.',
  },
  ...(SEASONS.length > 1
    ? [
        {
          target: 'season',
          title: 'Season',
          body: 'Switch the active season — items, trees and calculations follow it. Each season switch resets your incarnation/ether tree',
        },
      ]
    : []),
  {
    target: 'bottombar',
    title: 'Status Bar',
    body: 'Save status lives here — auto-save is on by default, Ctrl+S saves manually. Version, changelog and update checks too.',
  },
  {
    target: 'builds',
    title: 'Build Library',
    body: 'Save, organize, tag and switch between builds. Each build keeps its own gear, trees, notes and stash.',
  },
  {
    target: 'share',
    title: 'Share',
    body: 'Export your build as a code, gist or web link and import builds shared by others.',
  },
  {
    target: 'settings',
    title: 'Settings',
    body: 'App preferences — auto-save, number formatting and more.',
  },
  {
    title: 'Trust, but Verify',
    body: 'The DPS numbers here are approximations — the calculated damage does not always match what the game really deals. Always double-check your damage in-game.',
  },
  {
    title: "That's it!",
    body: 'Reopen this tour anytime with the ? button in the header. Happy planning!',
  },
]

export const CARD_WIDTH = 400
// first-frame fallback; the overlay passes the measured card height afterward
export const CARD_HEIGHT_ESTIMATE = 370
const CARD_GAP = 10
const EDGE_MARGIN = 12
const TALL_TARGET_RATIO = 0.55

export interface TargetRect {
  top: number
  left: number
  width: number
  height: number
}

export interface CardPlacement {
  placement: 'below' | 'above' | 'center'
  top: number
  left: number
}

export function placeCard(
  target: TargetRect | null,
  viewport: { width: number; height: number },
  cardHeight: number = CARD_HEIGHT_ESTIMATE,
): CardPlacement {
  if (!target || target.height > viewport.height * TALL_TARGET_RATIO) {
    return {
      placement: 'center',
      top: viewport.height / 2,
      left: viewport.width / 2,
    }
  }
  const left = Math.min(
    Math.max(EDGE_MARGIN, target.left + target.width / 2 - CARD_WIDTH / 2),
    Math.max(EDGE_MARGIN, viewport.width - CARD_WIDTH - EDGE_MARGIN),
  )
  const below = target.top + target.height + CARD_GAP
  if (below + cardHeight <= viewport.height - EDGE_MARGIN) {
    return { placement: 'below', top: below, left }
  }
  const above = target.top - CARD_GAP
  if (above - cardHeight >= EDGE_MARGIN) {
    return { placement: 'above', top: above, left }
  }
  return {
    placement: 'center',
    top: viewport.height / 2,
    left: viewport.width / 2,
  }
}
