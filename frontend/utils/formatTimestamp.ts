export function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso)
    if (Number.isNaN(d.getTime())) return iso
    const now = new Date()
    const time = d.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
    })
    const sameDate = (a: Date, b: Date) =>
      a.getFullYear() === b.getFullYear() &&
      a.getMonth() === b.getMonth() &&
      a.getDate() === b.getDate()
    if (sameDate(d, now)) return `Today, ${time}`
    const yesterday = new Date(now)
    yesterday.setDate(now.getDate() - 1)
    if (sameDate(d, yesterday)) return `Yesterday, ${time}`
    const day = d.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    })
    return `${day}, ${time}`
  } catch {
    return iso
  }
}
