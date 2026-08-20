interface StepModifiers {
  ctrlKey: boolean
  metaKey: boolean
  shiftKey: boolean
}

// one gesture set shared by attributes, skills and subskills
export function allocationStep(e: StepModifiers, cap: number): number {
  if ((e.ctrlKey || e.metaKey) && e.shiftKey) return cap
  if (e.shiftKey) return Math.min(5, cap)
  return 1
}
