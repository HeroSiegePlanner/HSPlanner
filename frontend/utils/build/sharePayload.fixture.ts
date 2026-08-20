import type { SharePayload } from './sharePayload'

export const SHARE_PAYLOAD_FIXTURE: SharePayload = {
  schemaVersion: 2,
  appVersion: '0.11.0-season-10',
  code: 'HSP://xK3fQ2aF0bT7pW9rL4dN8sV2jH6kM1yB5tG3zXqC0eU7aI9oP2mC9==',
  meta: {
    title: 'Fury of the Monsoon',
    className: 'Amazon',
    classId: 'amazon',
    level: 100,
    heroLevel: 40,
    seasonLabel: 'Season 10',
    tags: ['HC', '2P'],
    primaryAttribute: 'Dexterity',
    classIcon: 'classes/amazon.webp',
    dps: '165,4M',
  },
  profiles: [
    {
      name: 'Current',
      dps: '165,4M',
      statSections: [
        {
          title: 'Offense',
          rows: [
            { label: 'Hit DPS', value: '2,41M', tone: 'gold', glow: true },
            { label: 'Cast rate', value: '3,2/s' },
            { label: 'Crit chance', value: '38%' },
            { label: 'Crit damage', value: '+214%' },
          ],
        },
        {
          title: 'Defense',
          rows: [
            { label: 'Life', value: '6,950', tone: 'red' },
            { label: 'Armor', value: '1,204' },
            { label: 'Physical eHP', value: '12,701' },
            { label: 'Elemental eHP', value: '50,804' },
          ],
        },
        {
          title: 'Resistances',
          rows: [
            { label: 'Fire res', value: '75% (202)', tone: 'red' },
            { label: 'Cold res', value: '75% (202)', tone: 'cyan' },
            { label: 'Lightning res', value: '75% (504)', tone: 'orange' },
            { label: 'Poison res', value: '75% (202)', tone: 'green' },
          ],
        },
        {
          title: 'Attributes',
          rows: [
            { label: 'Strength', value: '104' },
            { label: 'Dexterity · Primary', value: '487', tone: 'gold' },
            { label: 'Intelligence', value: '92' },
            { label: 'Vitality', value: '358' },
          ],
        },
        {
          title: 'Sustain',
          rows: [
            { label: 'Mana', value: '2,515', tone: 'blue' },
            { label: 'Mana / cast', value: '42', tone: 'blue' },
            { label: 'Net mana / sec', value: '+18,4', tone: 'green' },
            { label: 'Life on hit', value: '+124', tone: 'green' },
          ],
        },
      ],
    },
  ],
  skills: {
    pointsLabel: '63 pts',
    groups: [
      {
        name: 'Storm',
        items: [
          {
            icon: 'skills/amazon/thunder_fury.png',
            name: 'Thunder Fury',
            type: 'Active · Lightning',
            main: true,
            rank: 12,
            max: 12,
            fromItems: '+4',
            effectiveRank: '16',
            desc: 'Hurls a storm spear that chains lightning between nearby enemies.',
            rows: [
              { label: 'Hit DPS', value: '118,2M', tone: 'gold', glow: true },
              { label: 'Mana / cast', value: '42', tone: 'blue' },
              { label: 'Chain targets', value: '5 (+2 tree)' },
            ],
            synergies: [
              { name: 'Envenom', text: '+12% damage per rank' },
              { name: "Thunder Goddess's Chosen", text: '+8% lightning damage per rank' },
            ],
          },
          {
            icon: 'skills/amazon/thunder_goddesses_chosen.png',
            name: "Thunder Goddess's Chosen",
            type: 'Passive',
            rank: 10,
            max: 10,
            desc: 'Astrope empowers you — your casts echo as bolts from the sky.',
            rows: [
              { label: 'Lightning damage', value: '+120%' },
              { label: 'Echo chance', value: '20%' },
            ],
          },
        ],
      },
      {
        name: 'Poison',
        items: [
          {
            icon: 'skills/amazon/envenom.png',
            name: 'Envenom',
            type: 'Active · Poison',
            rank: 12,
            max: 12,
            fromItems: '+2',
            effectiveRank: '14',
            desc: 'Coats your weapons in venom; hits apply a stacking poison.',
            rows: [
              { label: 'Hit DPS', value: '38,4M', tone: 'gold' },
              { label: 'Duration', value: '4,2s' },
            ],
            synergies: [{ name: 'Master Poisoner', text: '+9% poison duration per rank' }],
          },
          {
            icon: 'skills/amazon/noxious_strike.png',
            name: 'Noxious Strike',
            type: 'Active · Poison',
            rank: 8,
            max: 12,
            desc: 'A venomous thrust that bursts poison stacks on the target.',
          },
          {
            icon: 'skills/amazon/master_poisoner.png',
            name: 'Master Poisoner',
            type: 'Passive',
            rank: 10,
            max: 10,
            desc: 'You have studied every venom — poison serves you alone.',
          },
        ],
      },
      {
        name: 'Utility',
        items: [
          {
            icon: 'skills/amazon/storm_dash.png',
            name: 'Storm Dash',
            type: 'Active · Mobility',
            rank: 5,
            max: 12,
            desc: 'Dash forward on a bolt of wind, phasing through enemies.',
            rows: [
              { label: 'Cooldown', value: '0,8s' },
              { label: 'Charges', value: '2' },
            ],
          },
          {
            icon: 'skills/amazon/thrill_of_the_hunt.png',
            name: 'Thrill of the Hunt',
            type: 'Passive',
            rank: 6,
            max: 10,
            desc: 'Kills quicken your pulse — speed surges with every takedown.',
          },
        ],
      },
    ],
  },
  gear: {
    items: [
      {
        slot: 'Weapon',
        name: 'Stormlash, Fang of the Tempest',
        rarity: 'satanic',
        sockets: [
          { icon: 'socketable/Ber_spr.png' },
          { icon: 'socketable/Eth_spr.png', rainbow: true },
        ],
        tooltip: {
          name: 'Stormlash, Fang of the Tempest',
          rarity: 'satanic',
          typeLine: 'Satanic · Two-Handed Bow · 2-Handed · ★★★★',
          image: 'items/stormlash_fang_of_the_tempest.png',
          sections: [
            {
              lines: [
                { kind: 'row', label: 'Damage', value: '312–540' },
                { kind: 'row', label: 'Attacks / sec', value: '1,4' },
              ],
            },
            {
              header: { text: 'Implicit', tone: 'gold' },
              lines: [
                { kind: 'text', text: '+214–388 Lightning Damage', tone: 'gold' },
                { kind: 'text', text: '+42% Increased Attack Speed', tone: 'gold', badge: 'custom' },
              ],
            },
            {
              lines: [
                { kind: 'text', text: '+3 to Lightning Skills', tone: 'yellow' },
                { kind: 'text', text: '12% Chance to Cast Lv 20 Thunder Fury on Hit', tone: 'yellow' },
              ],
            },
            {
              header: { text: 'Unholy Affixes', tone: 'pink' },
              lines: [{ kind: 'text', text: '−8% Enemy Poison Resistance', tone: 'pink' }],
            },
            {
              header: { text: 'Granted Skill Effects', tone: 'orange' },
              lines: [
                {
                  kind: 'entry',
                  title: 'Wings of Hatred',
                  suffix: 'rank 4',
                  desc: 'Reduces landing diminish (capped at rank 1).',
                  lines: [],
                },
                {
                  kind: 'entry',
                  title: "Fallen God's Bloodlust",
                  suffix: 'rank 4-13',
                  desc: 'Converts a portion of your Attack Speed into Faster Cast Rate.',
                  lines: ['28–91% of Attack Speed added as Faster Cast Rate'],
                },
              ],
            },
            {
              header: { text: 'Forged · Satanic Crystal', tone: 'red' },
              lines: [{ kind: 'text', text: '+[10-25] to All Attributes', tone: 'red' }],
            },
            {
              header: { text: 'From Sockets', tone: 'gold' },
              lines: [{ kind: 'text', text: '+38% Enhanced Damage', tone: 'gold' }],
            },
            {
              header: { text: 'Monsoon Set', tone: 'green', trailing: '2/3 pieces' },
              lines: [
                { kind: 'entry', title: '2-Set (active)', tone: 'green', lines: ['+40% Lightning Damage'] },
                { kind: 'entry', title: '3-Set', tone: 'muted', lines: ['Storm Cloud follows you'] },
              ],
            },
            {
              lines: [
                { kind: 'entry', title: '12% Chance on Hit to Cast Lv 20 Thunder Fury', tone: 'good' },
              ],
            },
            {
              header: { text: 'Not Yet Supported', tone: 'muted' },
              lines: [{ kind: 'text', text: 'Attacks ignore weapon range', tone: 'muted' }],
              footnote: 'These mods are not yet calculated by the planner.',
            },
            {
              lines: [
                { kind: 'text', text: 'The storm does not ask permission.', tone: 'muted', italic: true },
              ],
            },
          ],
          footer: 'Req Level 92 · iLvl 90 · Tier S+',
        },
      },
      {
        slot: 'Helm',
        name: 'Crown of the Noxious Queen',
        rarity: 'mythic',
        sockets: [{ icon: 'socketable/Amethyst_spr.png' }],
        tooltip: {
          name: 'Crown of the Noxious Queen',
          rarity: 'mythic',
          typeLine: 'Mythic · Helmet · ★★',
          sections: [
            {
              header: { text: 'Implicit', tone: 'gold' },
              lines: [
                { kind: 'text', text: '+64 Dexterity', tone: 'gold' },
                { kind: 'text', text: '+31% Poison Duration', tone: 'gold' },
              ],
            },
            { lines: [{ kind: 'text', text: '+2 to Envenom', tone: 'yellow' }] },
          ],
          footer: 'Req Level 88 · Tier A',
        },
      },
    ],
  },
  incarnation: {
    countLabel: '96 / 1620 nodes',
    tabLabel: '96 nodes',
    keystones: [
      { name: 'Eye of the Storm', lines: ['Lightning skills arc to 2 additional enemies.', 'Chained hits deal 65% of base damage.'] },
      { name: 'Toxic Overload', lines: ['Poison effects can stack twice on the same target.', 'Poison duration reduced by 25%.'] },
      { name: "Windrunner's Grace", lines: ['+1% Movement Speed per 2% Attack Speed.', 'You cannot be slowed during Storm Dash.'] },
      { name: 'Chain Conduit', lines: ['Chains no longer diminish.', '+12% Lightning Damage per chained enemy (max 3).'] },
    ],
    notables: [
      { name: 'Stormcaller', line: '+24% Lightning Damage · +6% Cast Rate' },
      { name: 'Venom Ducts', line: '+20% Poison Damage · +10% Poison Duration' },
      { name: 'Fleetfoot', line: '+8% Movement Speed · +12 Evasion' },
      { name: "Serpent's Kiss", line: '+9% Poison Penetration' },
      { name: 'Skyfire', line: '+10% Critical Chance vs enemies above 80% life' },
      { name: 'Ionized Skin', line: '+20 Lightning Resistance · +3% Damage Reduction' },
      { name: "Hunter's Instinct", line: '+14% Attack Rating · +6% Thrill of the Hunt effect' },
      { name: 'Deadly Momentum', line: '+1% Attack Speed per kill, up to 10% · 5s' },
    ],
    minors: [
      { text: '+8 Dexterity', count: 14 },
      { text: '+6% Lightning Damage', count: 14 },
      { text: '+40 Life', count: 10 },
      { text: '+6% Poison Damage', count: 9 },
      { text: '+3% Attack Speed', count: 8 },
      { text: '+2% Evasion', count: 7 },
      { text: '+25 Mana', count: 6 },
      { text: '+2 All Resistances', count: 6 },
      { text: '+1% Critical Chance', count: 5 },
      { text: '+2% Movement Speed', count: 3 },
    ],
    jewelry: [
      { name: 'Exan Jewel', icon: 'socketable/Exan_Jewel_spr.png', line: '+14% Lightning Damage · +8% Cast Rate' },
      { name: 'Aether Jewel', icon: 'socketable/Aether_Jewel_spr.png', line: '+11% Attack Speed' },
    ],
    summaryLabel: '96 nodes · 4 keystones · 8 notables · 82 minors · 2 jewelry',
  },
  notes:
    'Standard S10 ladder starter. Cap lightning res before Act 4 — Coil of Static covers the gap until charms.',
}
