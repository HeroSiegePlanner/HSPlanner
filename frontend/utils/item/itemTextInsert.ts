import { SEP } from './itemTextShared'

const isSeparator = (line: string) => /^-{4,}$/.test(line.trim())

export function customImplicitLine(stat: {
  name: string
  format?: 'flat' | 'percent'
}): string {
  const suffix = stat.format === 'percent' ? '%' : ''
  return `+1${suffix} ${stat.name} [custom]`
}

export function insertImplicitLine(
  text: string,
  line: string,
): { text: string; offset: number } {
  const lines = text.split('\n')
  const implicitIdx = lines.findIndex((l) => l.trim() === 'Implicit:')
  const affixIdx = lines.findIndex((l) => l.trim() === 'Affixes:')

  let insertAt: number
  let block: string[]
  if (implicitIdx >= 0) {
    let end = implicitIdx + 1
    while (end < lines.length && !isSeparator(lines[end]!)) end++
    while (end > implicitIdx + 1 && lines[end - 1]!.trim() === '') end--
    insertAt = end
    block = [line]
  } else if (affixIdx >= 0) {
    insertAt = affixIdx
    block = ['Implicit:', line, SEP]
  } else {
    insertAt = lines.length
    block = [SEP, 'Implicit:', line]
  }

  const next = [...lines.slice(0, insertAt), ...block, ...lines.slice(insertAt)]
  const lineIdx = insertAt + block.indexOf(line)
  const offset = next.slice(0, lineIdx).reduce((n, l) => n + l.length + 1, 0)
  return { text: next.join('\n'), offset }
}
