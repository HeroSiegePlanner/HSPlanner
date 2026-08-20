export function isImageUrl(s: string | undefined): boolean {
  if (!s) return false
  return (
    /^https?:\/\//.test(s) ||
    /^data:image\//i.test(s) ||
    /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(s)
  )
}
