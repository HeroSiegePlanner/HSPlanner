import type { MouseEvent } from 'react'
import { inTauriRuntime } from './installUpdate'

export function openExternalLink(e: MouseEvent, href: string): void {
  if (!inTauriRuntime()) return
  e.preventDefault()
  void import('@tauri-apps/plugin-opener').then(({ openUrl }) => openUrl(href))
}
