import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import fishGif from '../../assets/tutorial-fish.gif'
import type { Section } from '../../App'
import { useDialogA11y } from '../../hooks/useDialogA11y'
import { writeStorage } from '../../utils/storage'
import {
  CARD_HEIGHT_ESTIMATE,
  CARD_WIDTH,
  TUTORIAL_DONE_KEY,
  TUTORIAL_STEPS,
  placeCard,
  type TargetRect,
  type TutorialStep,
} from './tutorialModel'

const SPOTLIGHT_PAD = 6
// ~1s of rAF retries: outlives the tab-switch exit/enter animation
const FIND_TRIES = 60
const EMPTY_STEP: TutorialStep = { title: '', body: '' }
const DIM_BG = 'rgba(0,0,0,0.72)'
const COMIC_BTN_CLASS =
  'rounded border-2 border-black bg-white px-3 py-1 font-mono text-[11px] font-bold uppercase tracking-[0.08em] text-black transition-colors hover:bg-black hover:text-white disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:bg-white disabled:hover:text-black'
const COMIC_BTN_PRIMARY_CLASS =
  'rounded border-2 border-black bg-black px-3 py-1 font-mono text-[11px] font-bold uppercase tracking-[0.08em] text-white transition-colors hover:bg-white hover:text-black'

interface TutorialOverlayProps {
  section: Section
  setSection: (section: Section) => void
  onClose: () => void
}

export default function TutorialOverlay({
  section,
  setSection,
  onClose,
}: TutorialOverlayProps) {
  const [index, setIndex] = useState(0)
  const [rect, setRect] = useState<TargetRect | null>(null)
  const [viewTick, setViewTick] = useState(0)
  const [cardHeight, setCardHeight] = useState(CARD_HEIGHT_ESTIMATE)
  const cardRef = useRef<HTMLDivElement>(null)
  const [startSection] = useState(section)
  const headingId = useId()

  const step = TUTORIAL_STEPS[index] ?? EMPTY_STEP
  const isLast = index === TUTORIAL_STEPS.length - 1

  const close = () => {
    writeStorage(TUTORIAL_DONE_KEY, '1')
    setSection(startSection)
    // scrollIntoView may pan the document to reach header buttons clipped at narrow widths
    if (window.scrollX || window.scrollY) window.scrollTo(0, 0)
    onClose()
  }

  useDialogA11y(cardRef, close)

  useEffect(() => {
    if (step.section) setSection(step.section)
    let raf = 0
    let tries = 0
    let acted = false
    const click = (selector: string) =>
      document
        .querySelector(selector)
        ?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }))
    const find = () => {
      if (step.act && !acted && document.querySelector(step.act)) {
        click(step.act)
        acted = true
      }
      if (!step.target) {
        setRect(null)
        return
      }
      const el = document.querySelector(`[data-tour="${step.target}"]`)
      el?.scrollIntoView?.({ block: 'nearest', inline: 'nearest' })
      const r = el?.getBoundingClientRect()
      if (r && r.width > 0) {
        setRect({ top: r.top, left: r.left, width: r.width, height: r.height })
      } else if (++tries < FIND_TRIES) {
        raf = requestAnimationFrame(find)
      } else {
        setRect(null)
      }
    }
    raf = requestAnimationFrame(find)
    return () => {
      cancelAnimationFrame(raf)
      if (acted && step.undo) click(step.undo)
    }
  }, [step, setSection, viewTick])

  useEffect(() => {
    const onResize = () => setViewTick((t) => t + 1)
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])

  // real height varies per step (body wrap); estimate only seeds the first frame
  useLayoutEffect(() => {
    const h = cardRef.current?.offsetHeight
    if (h) setCardHeight(h)
  }, [index, viewTick])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'ArrowRight') {
        e.preventDefault()
        setIndex((i) => Math.min(i + 1, TUTORIAL_STEPS.length - 1))
      } else if (e.key === 'ArrowLeft') {
        e.preventDefault()
        setIndex((i) => Math.max(0, i - 1))
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const viewport = { width: window.innerWidth, height: window.innerHeight }
  const hole = rect
    ? {
        top: rect.top - SPOTLIGHT_PAD,
        left: rect.left - SPOTLIGHT_PAD,
        right: rect.left + rect.width + SPOTLIGHT_PAD,
        bottom: rect.top + rect.height + SPOTLIGHT_PAD,
      }
    : null
  const pos = placeCard(rect, viewport, cardHeight)
  const transform =
    pos.placement === 'above'
      ? 'translateY(-100%)'
      : pos.placement === 'center'
        ? 'translate(-50%, -50%)'
        : undefined

  const tree = (
    // above app modals (z-100): tour steps open pickers/overlays and spotlight them
    <div role="presentation" className="fixed inset-0 z-[110]">
      {hole ? (
        // four dim strips around the hole: a 100vmax+ box-shadow trick dies on
        // GPU texture limits at large viewports, plain divs never do
        <>
          <div
            aria-hidden
            className="absolute transition-all duration-200"
            style={{
              top: 0,
              left: 0,
              right: 0,
              height: Math.max(0, hole.top),
              background: DIM_BG,
            }}
          />
          <div
            aria-hidden
            className="absolute transition-all duration-200"
            style={{ top: hole.bottom, left: 0, right: 0, bottom: 0, background: DIM_BG }}
          />
          <div
            aria-hidden
            className="absolute transition-all duration-200"
            style={{
              top: hole.top,
              left: 0,
              width: Math.max(0, hole.left),
              height: hole.bottom - hole.top,
              background: DIM_BG,
            }}
          />
          <div
            aria-hidden
            className="absolute transition-all duration-200"
            style={{
              top: hole.top,
              left: hole.right,
              right: 0,
              height: hole.bottom - hole.top,
              background: DIM_BG,
            }}
          />
          <div
            aria-hidden
            className="absolute rounded-md border border-accent-hot/70 transition-all duration-200"
            style={{
              top: hole.top,
              left: hole.left,
              width: hole.right - hole.left,
              height: hole.bottom - hole.top,
              boxShadow: '0 0 24px rgba(224,184,100,0.3)',
            }}
          />
        </>
      ) : (
        <div
          aria-hidden
          className="absolute inset-0"
          style={{ background: DIM_BG }}
        />
      )}

      <div
        ref={cardRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby={headingId}
        className="absolute flex flex-col outline-none transition-[top,left,transform] duration-200"
        style={{ top: pos.top, left: pos.left, width: CARD_WIDTH, transform }}
      >
        <div className="relative">
          {/* comic speech bubble, on purpose off-theme */}
          <div
            className="flex flex-col items-center border-[3px] border-black bg-white px-14 pb-10 pt-8 text-center"
            style={{
              borderRadius: '50%',
              boxShadow: '0 20px 60px rgba(0,0,0,0.55)',
            }}
          >
            <div className="mb-1.5 font-mono text-[10px] uppercase tracking-[0.12em] text-black/60">
              Tutorial · {index + 1} / {TUTORIAL_STEPS.length}
            </div>
            <h2
              id={headingId}
              className="m-0 text-[15px] font-bold tracking-[-0.01em] text-black"
            >
              {step.title}
            </h2>
            <p className="mb-0 mt-1.5 text-[12px] leading-relaxed text-black/70">
              {step.body}
            </p>
            <div className="mt-3 flex items-center gap-2">
              <button
                type="button"
                onClick={() => setIndex(Math.max(0, index - 1))}
                disabled={index === 0}
                className={COMIC_BTN_CLASS}
              >
                Back
              </button>
              <button
                type="button"
                onClick={() => (isLast ? close() : setIndex(index + 1))}
                className={COMIC_BTN_PRIMARY_CLASS}
              >
                {isLast ? 'Finish' : 'Next'}
              </button>
            </div>
            <button
              type="button"
              onClick={close}
              className="mt-2.5 font-mono text-[10px] uppercase tracking-[0.14em] text-black/60 transition-colors hover:text-black"
            >
              Skip tour
            </button>
          </div>
          {/* tail overlaps the oval: white fill erases the outline at the joint, stroke draws only
              the sides; px sizes only — rem classes shrink at html 13px and detach the joint */}
          <svg
            aria-hidden
            className="absolute -bottom-[34px] left-[120px] h-[64px] w-[40px]"
            viewBox="0 0 40 64"
          >
            <path
              d="M12 2 C16 22, 6 44, 0 62 C20 50, 32 24, 36 0 Z"
              fill="#fff"
            />
            <path
              d="M12.1 21.6 C10 35.7, 4.1 49.8, 0 62 C13.4 54, 23.2 39.6, 29.4 23.8"
              fill="none"
              stroke="#000"
              strokeWidth="3"
              strokeLinecap="round"
            />
          </svg>
        </div>
        <img
          src={fishGif}
          alt=""
          aria-hidden
          width={384}
          height={128}
          className="mt-3 w-[300px] self-start"
        />
      </div>
    </div>
  )

  return createPortal(tree, document.body)
}
