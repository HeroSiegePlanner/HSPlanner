import { z } from 'zod'

export const CURRENT_SCHEMA_VERSION = 2 as const
export const MAX_CODE_BYTES = 128 * 1024
export const MAX_SNAPSHOT_BYTES = 256 * 1024
export const MAX_TITLE_LENGTH = 128
export const MAX_CROCKFORD_ID_LENGTH = 6

export const CROCKFORD_ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
export const BUILD_ID_RE = /^[0-9A-HJKMNP-TV-Z]{6}$/

export const TONE_VALUES = [
  'gold',
  'red',
  'green',
  'blue',
  'cyan',
  'orange',
  'purple',
  'muted',
  'good',
  'yellow',
  'pink',
] as const
export const toneSchema = z.enum(TONE_VALUES)
export type Tone = z.infer<typeof toneSchema>

export const RARITY_VALUES = [
  'common',
  'uncommon',
  'rare',
  'mythic',
  'satanic',
  'heroic',
  'angelic',
  'satanic_set',
  'unholy',
  'relic',
] as const
export const raritySchema = z.enum(RARITY_VALUES)
export type Rarity = z.infer<typeof raritySchema>

function byteLength(s: string): number {
  return new TextEncoder().encode(s).length
}

const codeSchema = z.string().refine((s) => byteLength(s) <= MAX_CODE_BYTES, {
  message: `code exceeds ${MAX_CODE_BYTES} bytes`,
})

const statRowSchema = z.object({
  label: z.string(),
  value: z.string(),
  tone: toneSchema.optional(),
  glow: z.boolean().optional(),
})
export type StatRow = z.infer<typeof statRowSchema>

const statSectionSchema = z.object({
  title: z.string(),
  rows: z.array(statRowSchema),
})
export type StatSection = z.infer<typeof statSectionSchema>

const profileSchema = z.object({
  name: z.string(),
  dps: z.string(),
  statSections: z.array(statSectionSchema),
})
export type Profile = z.infer<typeof profileSchema>

const skillSynergySchema = z.object({
  name: z.string(),
  text: z.string(),
})
export type SkillSynergy = z.infer<typeof skillSynergySchema>

const skillItemSchema = z.object({
  icon: z.string().optional(),
  name: z.string(),
  type: z.string(),
  main: z.boolean().optional(),
  rank: z.number().int().min(0),
  max: z.number().int().min(1),
  fromItems: z.string().optional(),
  effectiveRank: z.string().optional(),
  desc: z.string().optional(),
  rows: z.array(statRowSchema).optional(),
  synergies: z.array(skillSynergySchema).optional(),
})
export type SkillItem = z.infer<typeof skillItemSchema>

const skillGroupSchema = z.object({
  name: z.string(),
  items: z.array(skillItemSchema),
})
export type SkillGroup = z.infer<typeof skillGroupSchema>

const skillsSchema = z.object({
  pointsLabel: z.string().optional(),
  groups: z.array(skillGroupSchema),
})

const gearSocketSchema = z.object({ icon: z.string(), rainbow: z.boolean().optional() })

const gearTooltipLineSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('row'), label: z.string(), value: z.string() }),
  z.object({
    kind: z.literal('text'),
    text: z.string(),
    tone: toneSchema.optional(),
    italic: z.boolean().optional(),
    small: z.boolean().optional(),
    badge: z.string().optional(),
  }),
  z.object({
    kind: z.literal('entry'),
    title: z.string(),
    suffix: z.string().optional(),
    desc: z.string().optional(),
    lines: z.array(z.string()).optional(),
    tone: toneSchema.optional(),
  }),
])
export type GearTooltipLine = z.infer<typeof gearTooltipLineSchema>

const gearTooltipSectionSchema = z.object({
  header: z
    .object({
      text: z.string(),
      tone: toneSchema.optional(),
      trailing: z.string().optional(),
    })
    .optional(),
  lines: z.array(gearTooltipLineSchema),
  footnote: z.string().optional(),
})
export type GearTooltipSection = z.infer<typeof gearTooltipSectionSchema>

const gearTooltipSchema = z.object({
  name: z.string(),
  rarity: raritySchema,
  typeLine: z.string(),
  image: z.string().optional(),
  sections: z.array(gearTooltipSectionSchema),
  footer: z.string().optional(),
})
export type GearTooltip = z.infer<typeof gearTooltipSchema>

const gearItemSchema = z.object({
  slot: z.string(),
  name: z.string(),
  rarity: raritySchema,
  sockets: z.array(gearSocketSchema),
  tooltip: gearTooltipSchema,
})
export type GearItem = z.infer<typeof gearItemSchema>

const gearSchema = z.object({
  items: z.array(gearItemSchema),
})

const branchSchema = z.object({
  name: z.string(),
  pts: z.number(),
  cap: z.number(),
  tone: toneSchema,
})
export type Branch = z.infer<typeof branchSchema>

const incarnationKeystoneSchema = z.object({
  name: z.string(),
  lines: z.array(z.string()),
})
const incarnationNotableSchema = z.object({
  name: z.string(),
  line: z.string(),
})
const incarnationMinorSchema = z.object({
  text: z.string(),
  count: z.number().int().min(1),
})
const incarnationJewelrySchema = z.object({
  name: z.string(),
  icon: z.string().optional(),
  line: z.string(),
})
export type IncarnationJewelry = z.infer<typeof incarnationJewelrySchema>

const incarnationSchema = z.object({
  countLabel: z.string(),
  tabLabel: z.string().optional(),
  branches: z.array(branchSchema).optional(),
  keystones: z.array(incarnationKeystoneSchema),
  notables: z.array(incarnationNotableSchema),
  minors: z.array(incarnationMinorSchema),
  jewelry: z.array(incarnationJewelrySchema),
  summaryLabel: z.string().optional(),
})
export type Incarnation = z.infer<typeof incarnationSchema>

const metaSchema = z.object({
  title: z.string().max(MAX_TITLE_LENGTH),
  className: z.string(),
  classId: z.string(),
  level: z.number(),
  heroLevel: z.number().optional(),
  seasonLabel: z.string(),
  tags: z.array(z.string()),
  primaryAttribute: z.string().optional(),
  classIcon: z.string().optional(),
  dps: z.string(),
})
export type Meta = z.infer<typeof metaSchema>

export const sharePayloadSchema = z.object({
  schemaVersion: z.literal(CURRENT_SCHEMA_VERSION),
  appVersion: z.string(),
  code: codeSchema,
  meta: metaSchema,
  profiles: z.array(profileSchema).min(1),
  skills: skillsSchema.optional(),
  gear: gearSchema.optional(),
  incarnation: incarnationSchema.optional(),
  notes: z.string().optional(),
})
export type SharePayload = z.infer<typeof sharePayloadSchema>

export function snapshotByteSize(payload: SharePayload): number {
  const { code, ...rest } = payload
  void code
  return byteLength(JSON.stringify(rest))
}
