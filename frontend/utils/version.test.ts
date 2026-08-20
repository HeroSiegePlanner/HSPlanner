import { describe, expect, it } from 'vitest'
import {
  compareSemver,
  formatBytes,
  mapReleaseToUpdateInfo,
  parseChangelog,
  shortSha,
} from './version'

describe('compareSemver', () => {
  it('orders simple versions', () => {
    expect(compareSemver('1.0.0', '0.9.9')).toBe(1)
    expect(compareSemver('0.4.2', '0.4.2')).toBe(0)
    expect(compareSemver('0.4.2', '0.5.0')).toBe(-1)
  })

  it('strips pre-release suffixes', () => {
    expect(compareSemver('1.0.0-rc.1', '1.0.0')).toBe(0)
  })
})

describe('parseChangelog', () => {
  const sample = `## New
- Ascendancy trees for all 6 classes
- Build sharing via short URL

## Improved
- Tree renderer is ~3x faster

## Balance
- Eagle Eye notable: +35% projectile damage → +28%

## Fixes
- Allocated nodes were sometimes deallocated
- Notes editor lost cursor position`

  it('extracts tagged sections in order', () => {
    const sections = parseChangelog(sample)
    expect(sections).toHaveLength(4)
    expect(sections.map((s) => s.tag)).toEqual([
      'new',
      'improved',
      'balance',
      'fixes',
    ])
  })

  it('captures bullets per section', () => {
    const sections = parseChangelog(sample)
    expect(sections[0]!.items).toHaveLength(2)
    expect(sections[0]!.items[0]).toBe('Ascendancy trees for all 6 classes')
    expect(sections[3]!.items[1]).toContain('cursor position')
  })

  it('drops empty sections', () => {
    const sections = parseChangelog('## Improved\n## Fixes\n- a thing')
    expect(sections).toHaveLength(1)
    expect(sections[0]!.tag).toBe('fixes')
  })

  it('treats unknown headings as "other"', () => {
    const sections = parseChangelog('## Misc\n- one\n- two')
    expect(sections[0]!.tag).toBe('other')
    expect(sections[0]!.items).toHaveLength(2)
  })
})

describe('formatBytes', () => {
  it('formats bytes through GB', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(1023)).toBe('1023 B')
    expect(formatBytes(1024)).toBe('1.00 KB')
    expect(formatBytes(12_400_000)).toMatch(/MB$/)
    expect(formatBytes(1024 * 1024 * 1024)).toMatch(/GB$/)
  })
})

describe('shortSha', () => {
  it('shortens long hashes', () => {
    expect(shortSha('3a8f1234567890abcdef1234567890abcdef1234d214')).toBe(
      '3a8f…d214',
    )
  })
  it('strips sha256: prefix', () => {
    expect(shortSha('sha256:abcdef1234567890abcdef1234567890abcdef12')).toBe(
      'abcd…ef12',
    )
  })
})

describe('mapReleaseToUpdateInfo', () => {
  it('flags updates and extracts asset metadata', () => {
    const info = mapReleaseToUpdateInfo({
      tag_name: 'v9.9.9',
      name: 'Big release',
      html_url: 'https://example.test/r',
      body: '## New\n- Cool thing',
      published_at: '2026-04-28T14:30:00Z',
      assets: [
        {
          name: 'HSPlanner.dmg',
          size: 12_400_000,
          digest: 'sha256:3a8fabcdef1234567890abcdef1234567890abcdef1234d214',
          browser_download_url: 'https://example.test/asset',
        },
      ],
    })
    expect(info.hasUpdate).toBe(true)
    expect(info.latest).toBe('9.9.9')
    expect(info.releaseUrl).toBe('https://example.test/r')
    expect(info.body).toContain('Cool thing')
    expect(info.assetSize).toBe(12_400_000)
    expect(info.assetSha).toMatch(/^3a8f/)
    expect(info.assetUrl).toBe('https://example.test/asset')
  })
})
