import { SNAPSHOT_KEYS } from './autoSave'
import { useBuild } from './build'
import type { BuildStore } from './build/types'

export const UNDO_LIMIT = 50

type UndoSnapshot = Pick<BuildStore, (typeof SNAPSHOT_KEYS)[number]>

let undoStack: UndoSnapshot[] = []
let redoStack: UndoSnapshot[] = []
let isRestoring = false

function pickSnapshot(state: BuildStore): UndoSnapshot {
  const snap: Record<string, unknown> = {}
  for (const key of SNAPSHOT_KEYS) snap[key] = state[key]
  return snap as UndoSnapshot
}

export function initUndoHistory(): () => void {
  undoStack = []
  redoStack = []
  const unsubscribe = useBuild.subscribe((state, prev) => {
    if (isRestoring) return
    if (
      state.activeBuildId !== prev.activeBuildId ||
      state.activeProfileId !== prev.activeProfileId
    ) {
      // history from another build must not leak into the one just opened
      undoStack = []
      redoStack = []
      return
    }
    if (!SNAPSHOT_KEYS.some((k) => state[k] !== prev[k])) return
    undoStack = [...undoStack.slice(-(UNDO_LIMIT - 1)), pickSnapshot(prev)]
    redoStack = []
  })
  return () => {
    unsubscribe()
    undoStack = []
    redoStack = []
  }
}

function restore(snapshot: UndoSnapshot): void {
  isRestoring = true
  try {
    useBuild.setState(snapshot)
  } finally {
    isRestoring = false
  }
}

export function undoLastChange(): boolean {
  const snapshot = undoStack[undoStack.length - 1]
  if (!snapshot) return false
  undoStack = undoStack.slice(0, -1)
  redoStack = [...redoStack, pickSnapshot(useBuild.getState())]
  restore(snapshot)
  return true
}

export function redoLastChange(): boolean {
  const snapshot = redoStack[redoStack.length - 1]
  if (!snapshot) return false
  redoStack = redoStack.slice(0, -1)
  undoStack = [...undoStack, pickSnapshot(useBuild.getState())]
  restore(snapshot)
  return true
}
