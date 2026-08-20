import { inTauriRuntime } from './installUpdate'

export const UI_ZOOM_STEPS = [1, 1.15, 1.25, 1.5, 1.75, 2] as const

export type UiZoom = (typeof UI_ZOOM_STEPS)[number]

export function isUiZoom(value: unknown): value is UiZoom {
  return UI_ZOOM_STEPS.includes(value as UiZoom)
}

// screen.width is CSS px, so a display already scaled by the OS reports the
// smaller number and stays at 100%
export function autoUiZoom(screenWidth: number): UiZoom {
  if (screenWidth >= 3400) return 1.5
  if (screenWidth >= 2400) return 1.25
  return 1
}

export function detectUiZoom(): UiZoom {
  if (typeof window === 'undefined' || !window.screen) return 1
  return autoUiZoom(window.screen.width)
}

export function applyUiZoom(zoom: UiZoom): void {
  if (!inTauriRuntime()) return
  void import('@tauri-apps/api/webview')
    .then(({ getCurrentWebview }) => getCurrentWebview().setZoom(zoom))
    .catch((err) => console.error('setZoom failed', err))
}
