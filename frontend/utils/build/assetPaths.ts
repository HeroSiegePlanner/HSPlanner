
const CLASS_ICON_FILES = import.meta.glob<string>('../../assets/classes/*.{png,webp,jpg,jpeg}', {
  eager: true,
  query: '?url',
  import: 'default',
})
const CLASS_ICON_PATH: Record<string, string> = {}
for (const p of Object.keys(CLASS_ICON_FILES)) {
  const file = p.split('/').pop() ?? ''
  const classId = file.replace(/\.(png|webp|jpe?g)$/i, '')
  if (classId) CLASS_ICON_PATH[classId] = `classes/${file}`
}
export function classIconPath(classId: string): string | undefined {
  return CLASS_ICON_PATH[classId]
}

const SKILL_ICON_FILES = import.meta.glob<string>('../../assets/skills/**/*.{png,webp,jpg,jpeg}', {
  eager: true,
  query: '?url',
  import: 'default',
})
const SKILL_ICON_PATH: Record<string, string> = {}
for (const p of Object.keys(SKILL_ICON_FILES)) {
  const parts = p.split('/')
  const file = parts.pop() ?? ''
  const classDir = parts.pop() ?? ''
  const skillId = file.replace(/\.(png|webp|jpe?g)$/i, '')
  if (classDir && skillId) SKILL_ICON_PATH[`${classDir}/${skillId}`] = `skills/${classDir}/${file}`
}
export function skillIconPath(classId: string, skillId: string): string | undefined {
  return SKILL_ICON_PATH[`${classId}/${skillId}`]
}

const SOCKETABLE_ICON_FILES = import.meta.glob<string>('../../assets/socketable/*.png', {
  eager: true,
  query: '?url',
  import: 'default',
})
const SOCKETABLE_ICON_PATH: Record<string, string> = {}
for (const p of Object.keys(SOCKETABLE_ICON_FILES)) {
  const file = p.split('/').pop() ?? ''
  const key = file.replace(/_spr\.png$/i, '').replace(/_/g, ' ').toLowerCase()
  SOCKETABLE_ICON_PATH[key] = `socketable/${file}`
}
export function socketIconPath(name: string): string | undefined {
  return SOCKETABLE_ICON_PATH[name.toLowerCase()]
}

export function itemImagePath(itemId: string): string {
  return `items/${itemId}.png`
}
