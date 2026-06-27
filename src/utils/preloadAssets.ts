const collectUrls = (map: Record<string, string>): string[] => Object.values(map)

const SPRITE_URLS: string[] = [
  ...collectUrls(
    import.meta.glob<string>('../assets/items/*.{png,webp,jpg,jpeg}', {
      eager: true,
      query: '?url',
      import: 'default',
    }),
  ),
  ...collectUrls(
    import.meta.glob<string>('../assets/skills/**/*.{png,webp,jpg,jpeg}', {
      eager: true,
      query: '?url',
      import: 'default',
    }),
  ),
  ...collectUrls(
    import.meta.glob<string>('../assets/atlas/**/*.{png,webp,jpg,jpeg}', {
      eager: true,
      query: '?url',
      import: 'default',
    }),
  ),
  ...collectUrls(
    import.meta.glob<string>(
      '../assets/socketable/**/*.{png,webp,jpg,jpeg}',
      { eager: true, query: '?url', import: 'default' },
    ),
  ),
  ...collectUrls(
    import.meta.glob<string>(
      '../assets/subskills/**/*.{png,webp,jpg,jpeg}',
      { eager: true, query: '?url', import: 'default' },
    ),
  ),
  ...collectUrls(
    import.meta.glob<string>('../assets/*.{png,webp,jpg,jpeg}', {
      eager: true,
      query: '?url',
      import: 'default',
    }),
  ),
]

export function preloadSprites(
  onProgress?: (loaded: number, total: number) => void,
): Promise<void> {
  const total = SPRITE_URLS.length
  if (total === 0) {
    onProgress?.(0, 0)
    return Promise.resolve()
  }
  let done = 0
  return new Promise((resolve) => {
    const finish = () => {
      done += 1
      onProgress?.(done, total)
      if (done >= total) resolve()
    }
    for (const url of SPRITE_URLS) {
      const img = new Image()
      img.onload = finish
      img.onerror = finish
      img.src = url
    }
  })
}
