import { create } from 'zustand'
import { NUMBER_SCALES, type NumberScale } from '../utils/compactNumber'
import { readStorage, writeStorage } from '../utils/storage'

const SETTINGS_KEY = 'hsplanner.settings.v1'

interface SettingsValues {
  autoSave: boolean
  numberScale: NumberScale
}

interface SettingsState extends SettingsValues {
  setAutoSave: (autoSave: boolean) => void
  setNumberScale: (numberScale: NumberScale) => void
}

const DEFAULTS: SettingsValues = { autoSave: true, numberScale: 'billions' }

function loadSettings(): SettingsValues {
  const raw = readStorage(SETTINGS_KEY)
  if (!raw) return DEFAULTS
  try {
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null) return DEFAULTS
    const o = parsed as Record<string, unknown>
    return {
      autoSave:
        typeof o.autoSave === 'boolean' ? o.autoSave : DEFAULTS.autoSave,
      numberScale: NUMBER_SCALES.includes(o.numberScale as NumberScale)
        ? (o.numberScale as NumberScale)
        : DEFAULTS.numberScale,
    }
  } catch {
    return DEFAULTS
  }
}

function persist(values: SettingsValues): void {
  writeStorage(
    SETTINGS_KEY,
    JSON.stringify({
      autoSave: values.autoSave,
      numberScale: values.numberScale,
    }),
  )
}

export const useSettings = create<SettingsState>((set, get) => ({
  ...loadSettings(),
  setAutoSave: (autoSave) => {
    set({ autoSave })
    persist(get())
  },
  setNumberScale: (numberScale) => {
    set({ numberScale })
    persist(get())
  },
}))
