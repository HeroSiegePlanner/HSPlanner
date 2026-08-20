function syncPct(input: HTMLInputElement): void {
  const min = Number(input.min) || 0
  const max = Number(input.max) || 100
  const span = max - min
  const v = Number(input.value)
  const pct = span === 0 ? 0 : ((v - min) / span) * 100
  input.style.setProperty('--sl-pct', pct + '%')
}

function syncIfRange(node: Node): void {
  if (!(node instanceof HTMLElement)) return
  if (node instanceof HTMLInputElement && node.type === 'range') {
    syncPct(node)
  }
  const ranges = node.querySelectorAll?.('input[type="range"]')
  ranges?.forEach((el) => syncPct(el as HTMLInputElement))
}

let initialized = false

export function initRangeInputs(): void {
  if (initialized || typeof document === 'undefined') return
  initialized = true

  const initialPass = () => {
    document.querySelectorAll('input[type="range"]').forEach((el) => {
      syncPct(el as HTMLInputElement)
    })
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initialPass, { once: true })
  } else {
    initialPass()
  }

  const onValueEvent = (e: Event) => {
    const t = e.target
    if (t instanceof HTMLInputElement && t.type === 'range') syncPct(t)
  }
  document.addEventListener('input', onValueEvent, true)
  document.addEventListener('change', onValueEvent, true)

  const observer = new MutationObserver((mutations) => {
    for (const m of mutations) {
      if (m.type === 'childList') {
        m.addedNodes.forEach(syncIfRange)
      } else if (
        m.type === 'attributes' &&
        m.target instanceof HTMLInputElement &&
        m.target.type === 'range' &&
        (m.attributeName === 'value' ||
          m.attributeName === 'min' ||
          m.attributeName === 'max')
      ) {
        syncPct(m.target)
      }
    }
  })

  if (document.body) {
    observer.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['value', 'min', 'max'],
    })
  } else {
    document.addEventListener(
      'DOMContentLoaded',
      () => {
        observer.observe(document.body, {
          childList: true,
          subtree: true,
          attributes: true,
          attributeFilter: ['value', 'min', 'max'],
        })
      },
      { once: true },
    )
  }
}
