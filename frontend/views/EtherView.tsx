import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import treeBackground from '../assets/atlas/Incarnation_Background.png'
import { useBuild } from '../store/build'
import { findPath, reachableFromAny } from '../utils/tree/treeGraph'
import {
  ETHER_ADJ,
  ETHER_NODE_BY_ID,
  ETHER_START_IDS,
  ETHER_START_SET,
} from '../utils/tree/etherGraph'
import {
  ETHER_EDGE_ALLOC,
  ETHER_EDGE_BASE,
  ETHER_EDGE_PREVIEW,
  ETHER_NODE_ICONS,
  ETHER_NODES,
  ETHER_EDGES,
  ETHER_SEARCH_INDEX,
  etherNodeFill,
  etherNodeStroke,
  etherStrokeWidth,
  vbH,
  vbW,
  vbX,
  vbY,
  type EtherNodePaint,
} from './ether/etherData'
import { EtherNodeTooltip } from './ether/EtherNodeTooltip'
import { EtherSummaryPanel } from './ether/EtherSummaryPanel'
import { zoomAtPoint } from '../utils/tree/viewTransform'
import LoadoutBar from '../components/LoadoutBar'
import ProgressionSlider from '../components/ProgressionSlider'
import { useProgressionPreview } from '../hooks/useProgressionPreview'

export default function EtherView() {
  const allocated = useBuild((s) => s.allocatedEtherNodes)
  const toggleNode = useBuild((s) => s.toggleEtherNode)
  const resetEther = useBuild((s) => s.resetEtherNodes)

  const [scale, setScale] = useState(0.6)
  const [tx, setTx] = useState(0)
  const [ty, setTy] = useState(0)
  const [hoverId, setHoverId] = useState<number | null>(null)
  const [hoverPos, setHoverPos] = useState<{ x: number; y: number } | null>(null)
  const [dragging, setDragging] = useState(false)
  const [viewportSize, setViewportSize] = useState({ w: 1000, h: 800 })
  const [searchQuery, setSearchQuery] = useState('')
  const [summaryOpen, setSummaryOpen] = useState(true)
  const { progressStep, setProgressStep, visibleAllocated, markerId } =
    useProgressionPreview(allocated, ETHER_START_SET, ETHER_ADJ)

  const containerRef = useRef<HTMLDivElement>(null)
  const dragStart = useRef<{ x: number; y: number; tx: number; ty: number } | null>(null)
  const didDragRef = useRef(false)

  const fitView = useCallback(() => {
    const el = containerRef.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    const fit = Math.min(rect.width / vbW, rect.height / vbH) * 0.95
    setScale(fit)
    setTx((rect.width - vbW * fit) / 2 - vbX * fit)
    setTy((rect.height - vbH * fit) / 2 - vbY * fit)
  }, [])

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const update = () => {
      const rect = el.getBoundingClientRect()
      setViewportSize({ w: rect.width || 1000, h: rect.height || 800 })
    }
    update()
    const ro = new ResizeObserver(update)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  useEffect(() => {
    fitView()
  }, [fitView])

  const scaleRef = useRef(scale)
  const txRef = useRef(tx)
  const tyRef = useRef(ty)
  useEffect(() => {
    scaleRef.current = scale
  }, [scale])
  useEffect(() => {
    txRef.current = tx
  }, [tx])
  useEffect(() => {
    tyRef.current = ty
  }, [ty])
  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const handler = (e: WheelEvent) => {
      e.preventDefault()
      const rect = el.getBoundingClientRect()
      const next = zoomAtPoint(
        { scale: scaleRef.current, tx: txRef.current, ty: tyRef.current },
        e.clientX - rect.left,
        e.clientY - rect.top,
        e.deltaY <= 0,
        0.2,
        6,
      )
      // sync refs now — wheel events can fire again before the next render
      scaleRef.current = next.scale
      txRef.current = next.tx
      tyRef.current = next.ty
      setScale(next.scale)
      setTx(next.tx)
      setTy(next.ty)
      // rebase an active drag, else the next mousemove snaps to the pre-zoom pan
      if (dragStart.current) {
        dragStart.current = { x: e.clientX, y: e.clientY, tx: next.tx, ty: next.ty }
      }
    }
    el.addEventListener('wheel', handler, { passive: false })
    return () => el.removeEventListener('wheel', handler)
  }, [])

  function onMouseDown(e: React.MouseEvent) {
    if (e.button !== 0) return
    setDragging(true)
    didDragRef.current = false
    dragStart.current = { x: e.clientX, y: e.clientY, tx, ty }
  }

  function onMouseMove(e: React.MouseEvent) {
    if (!dragging || !dragStart.current) return
    const dx = e.clientX - dragStart.current.x
    const dy = e.clientY - dragStart.current.y
    if (Math.abs(dx) + Math.abs(dy) > 3) didDragRef.current = true
    setTx(dragStart.current.tx + dx)
    setTy(dragStart.current.ty + dy)
  }

  function onMouseUp() {
    setDragging(false)
  }

  const progressMarkerNode = useMemo(
    () => (markerId != null ? ETHER_NODE_BY_ID.get(markerId) ?? null : null),
    [markerId],
  )

  const hoverNode = useMemo(
    () => (hoverId != null ? ETHER_NODE_BY_ID.get(hoverId) ?? null : null),
    [hoverId],
  )

  const previewPath = useMemo(() => {
    if (progressStep != null) return null
    if (hoverId == null || allocated.has(hoverId)) return null
    const sources = new Set<number>([...allocated, ...ETHER_START_IDS])
    const path = findPath(sources, hoverId, ETHER_ADJ)
    if (!path) return null
    return new Set(path)
  }, [progressStep, hoverId, allocated])

  const previewEdgeKeys = useMemo(() => {
    if (!previewPath) return null
    const keys = new Set<string>()
    const arr = [...previewPath]
    for (let i = 0; i < arr.length - 1; i++) {
      const a = arr[i]!
      const b = arr[i + 1]!
      keys.add(a < b ? `${a}-${b}` : `${b}-${a}`)
    }
    return keys
  }, [previewPath])

  const previewAddedCount = useMemo(() => {
    if (!previewPath) return 0
    let c = 0
    for (const id of previewPath) if (!allocated.has(id)) c++
    return c
  }, [allocated, previewPath])

  const previewRemovedCount = useMemo(() => {
    if (hoverId == null || !allocated.has(hoverId)) return 0
    const without = new Set(allocated)
    without.delete(hoverId)
    const remaining = reachableFromAny(ETHER_START_SET, without, ETHER_ADJ)
    return allocated.size - remaining.size
  }, [hoverId, allocated])

  const allocatedSameKey = useMemo(() => {
    if (!hoverNode) return 0
    let c = 0
    for (const id of allocated) {
      if (ETHER_NODE_BY_ID.get(id)?.key === hoverNode.key) c++
    }
    return c
  }, [hoverNode, allocated])

  const searchMatches = useMemo(() => {
    const q = searchQuery.trim().toLowerCase()
    if (!q) return null
    const matches = new Set<number>()
    for (const entry of ETHER_SEARCH_INDEX) {
      if (entry.haystack.includes(q)) matches.add(entry.id)
    }
    return matches
  }, [searchQuery])

  const allocatedEdgeKeys = useMemo(() => {
    const keys = new Set<string>()
    for (const id of visibleAllocated) {
      for (const nb of ETHER_ADJ.get(id) ?? []) {
        if (visibleAllocated.has(nb)) {
          keys.add(id < nb ? `${id}-${nb}` : `${nb}-${id}`)
        }
      }
    }
    return keys
  }, [visibleAllocated])

  const nodeCircles = useMemo(
    () =>
      ETHER_NODES.map((n) => {
        const paint: EtherNodePaint = {
          isAlloc: visibleAllocated.has(n.id),
          isPreview: false,
          isRoot: n.t === 'root',
          isBig: n.t === 'big',
        }
        return (
          <circle
            key={n.id}
            cx={n.x}
            cy={n.y}
            r={paint.isRoot ? n.r + 3 : n.r}
            fill={etherNodeFill(paint)}
            stroke={etherNodeStroke(paint)}
            strokeWidth={etherStrokeWidth(paint.isRoot, paint.isBig)}
            style={{ cursor: 'pointer' }}
            onMouseEnter={(e) => {
              setHoverId(n.id)
              setHoverPos({ x: e.clientX, y: e.clientY })
            }}
            onMouseMove={(e) => {
              setHoverPos({ x: e.clientX, y: e.clientY })
            }}
            onMouseLeave={() => {
              setHoverId((cur) => (cur === n.id ? null : cur))
              setHoverPos(null)
            }}
            onClick={(e) => {
              e.stopPropagation()
              if (didDragRef.current) {
                didDragRef.current = false
                return
              }
              if (progressStep != null) {
                setProgressStep(null)
                return
              }
              toggleNode(n.id)
            }}
          />
        )
      }),
    [visibleAllocated, toggleNode, progressStep, setProgressStep],
  )

  const previewOverlay = useMemo(() => {
    if (!previewPath) return null
    return ETHER_NODES.filter(
      (n) => previewPath.has(n.id) && !allocated.has(n.id),
    ).map((n) => {
      const paint: EtherNodePaint = {
        isAlloc: false,
        isPreview: true,
        isRoot: n.t === 'root',
        isBig: n.t === 'big',
      }
      return (
        <circle
          key={`preview-${n.id}`}
          cx={n.x}
          cy={n.y}
          r={n.r}
          fill={etherNodeFill(paint)}
          stroke={etherNodeStroke(paint)}
          strokeWidth={etherStrokeWidth(false, paint.isBig)}
          pointerEvents="none"
        />
      )
    })
  }, [previewPath, allocated])

  const matchOverlay = useMemo(() => {
    if (!searchMatches) return null
    return ETHER_NODES.filter((n) => searchMatches.has(n.id)).map((n) => (
      <circle
        key={`match-${n.id}`}
        cx={n.x}
        cy={n.y}
        r={n.r + 4}
        fill="none"
        stroke="#a574c9"
        strokeWidth={3}
      />
    ))
  }, [searchMatches])

  const edgeById = useCallback(
    (a: number, b: number) => {
      const key = a < b ? `${a}-${b}` : `${b}-${a}`
      if (allocatedEdgeKeys.has(key)) {
        return { stroke: ETHER_EDGE_ALLOC, strokeWidth: 2.5 }
      }
      if (previewEdgeKeys?.has(key)) {
        return { stroke: ETHER_EDGE_PREVIEW, strokeWidth: 2 }
      }
      return { stroke: ETHER_EDGE_BASE, strokeWidth: 1.5 }
    },
    [allocatedEdgeKeys, previewEdgeKeys],
  )

  return (
    <div className="relative h-full" style={{ backgroundColor: '#0a0b0f' }}>
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0"
        style={{
          backgroundImage: `url(${treeBackground})`,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          transform: 'translateZ(0)',
          willChange: 'transform',
        }}
      />
      <div
        ref={containerRef}
        className="absolute inset-0 overflow-hidden"
        style={{ cursor: dragging ? 'grabbing' : 'grab' }}
        onMouseDown={onMouseDown}
        onMouseMove={onMouseMove}
        onMouseUp={onMouseUp}
        onMouseLeave={onMouseUp}
      >
        <svg
          width="100%"
          height="100%"
          style={{ display: 'block' }}
          viewBox={`0 0 ${viewportSize.w} ${viewportSize.h}`}
        >
          <g transform={`translate(${tx} ${ty}) scale(${scale})`}>
            <g>
              {ETHER_EDGES.map(([a, b]) => {
                const na = ETHER_NODE_BY_ID.get(a)
                const nb = ETHER_NODE_BY_ID.get(b)
                if (!na || !nb) return null
                const paint = edgeById(a, b)
                return (
                  <line
                    key={`${a}-${b}`}
                    x1={na.x}
                    y1={na.y}
                    x2={nb.x}
                    y2={nb.y}
                    stroke={paint.stroke}
                    strokeWidth={paint.strokeWidth}
                  />
                )
              })}
            </g>
            <g style={{ opacity: searchMatches ? 0.25 : 1 }}>{nodeCircles}</g>
            {previewOverlay && (
              <g style={{ opacity: searchMatches ? 0.25 : 1 }}>
                {previewOverlay}
              </g>
            )}
            <g
              pointerEvents="none"
              style={{ opacity: searchMatches ? 0.25 : 1 }}
            >
              {ETHER_NODE_ICONS.map((icon) => {
                const size = icon.r * 2
                return (
                  <image
                    key={`icon-${icon.id}`}
                    href={icon.href}
                    x={icon.x - icon.r}
                    y={icon.y - icon.r}
                    width={size}
                    height={size}
                    preserveAspectRatio="xMidYMid meet"
                    style={{ imageRendering: 'pixelated' }}
                  />
                )
              })}
            </g>
            {matchOverlay && <g pointerEvents="none">{matchOverlay}</g>}
            {progressMarkerNode && (
              <circle
                pointerEvents="none"
                cx={progressMarkerNode.x}
                cy={progressMarkerNode.y}
                r={progressMarkerNode.r + 6}
                fill="none"
                stroke="#a574c9"
                strokeWidth={2.5}
                opacity={0.9}
                style={{ filter: 'drop-shadow(0 0 8px rgba(165,116,201,0.6))' }}
              />
            )}
          </g>
        </svg>
      </div>

      <div
        className="pointer-events-none absolute inset-0"
        style={{
          background:
            'radial-gradient(ellipse at 50% 50%, transparent 50%, rgba(0,0,0,0.55) 100%)',
        }}
      />

      <div className="pointer-events-none absolute right-3.5 top-3 z-10 flex flex-col items-end gap-2">
        <div className="flex items-start gap-1.5">
          <div className="pointer-events-auto relative">
            <svg
              aria-hidden
              className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-faint"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={2}
            >
              <circle cx="11" cy="11" r="7" />
              <path d="m20 20-3.5-3.5" />
            </svg>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search ether nodes…"
              data-search-input
              className="w-64 rounded-[3px] border border-border-2 px-3 py-1.5 pl-9 pr-14 font-mono text-[11px] text-text placeholder:text-faint transition-colors focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
              style={{
                background:
                  'linear-gradient(180deg, #0d0e12, var(--color-panel-2))',
                boxShadow: 'inset 0 1px 2px rgba(0,0,0,0.5)',
                backdropFilter: 'blur(4px)',
              }}
            />
            {searchQuery && (
              <>
                <span className="pointer-events-none absolute right-8 top-1/2 -translate-y-1/2 font-mono text-[10px] uppercase tracking-[0.14em] text-stat-purple">
                  {searchMatches?.size ?? 0}
                </span>
                <button
                  onClick={() => setSearchQuery('')}
                  className="absolute right-2 top-1/2 -translate-y-1/2 rounded-[2px] px-1 font-mono text-[12px] text-faint transition-colors hover:text-accent-hot"
                  aria-label="Clear search"
                >
                  ×
                </button>
              </>
            )}
          </div>
          <button
            onClick={() => setSummaryOpen((v) => !v)}
            className={`pointer-events-auto rounded-[3px] border px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] transition-colors ${
              summaryOpen
                ? 'border-stat-purple/60 text-stat-purple'
                : 'border-border-2 text-muted hover:border-stat-purple/60 hover:text-stat-purple'
            }`}
            style={{ background: 'linear-gradient(180deg, #221a2c, #16121c)' }}
          >
            Summary
          </button>
          <button
            onClick={fitView}
            className="pointer-events-auto rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Fit
          </button>
          <button
            onClick={resetEther}
            className="pointer-events-auto rounded-[3px] border border-border-2 bg-transparent px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-stat-red hover:text-stat-red"
          >
            Reset
          </button>
        </div>

        <LoadoutBar tab="ether" scopeLabel="Ether nodes" dataTour="ether-loadouts" />
        {summaryOpen && <EtherSummaryPanel allocated={allocated} />}
      </div>

      <div
        className="pointer-events-none absolute bottom-3.5 left-3.5 z-10 inline-flex items-center gap-3 rounded-[3px] border border-border px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint"
        style={{
          background:
            'linear-gradient(180deg, color-mix(in srgb, var(--color-panel-2) 80%, transparent), color-mix(in srgb, var(--color-bg) 70%, transparent))',
          backdropFilter: 'blur(6px)',
          boxShadow:
            'inset 0 1px 0 rgba(165,116,201,0.06), 0 4px 16px rgba(0,0,0,0.45)',
        }}
      >
        <span className="inline-flex items-center gap-1.5">
          <span
            aria-hidden
            className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
          />
          Nodes
          <span className="text-text">{ETHER_NODES.length}</span>
        </span>
        <span aria-hidden className="h-3 w-px bg-border" />
        <span className="inline-flex items-center gap-1.5">
          <span
            aria-hidden
            className="inline-block h-1 w-1 rotate-45 bg-stat-purple"
            style={{ boxShadow: '0 0 6px rgba(165,116,201,0.6)' }}
          />
          Allocated
          <span className="text-stat-purple">{allocated.size}</span>
        </span>
        <span aria-hidden className="h-3 w-px bg-border" />
        <span className="inline-flex items-center gap-1.5">
          Zoom
          <span className="text-stat-purple">{(scale * 100).toFixed(0)}%</span>
        </span>
      </div>

      <ProgressionSlider
        total={allocated.size}
        value={progressStep}
        onChange={setProgressStep}
        accentClass="text-stat-purple"
      />

      {hoverNode && hoverPos && !dragging &&
        createPortal(
          <EtherNodeTooltip
            key={hoverNode.id}
            node={hoverNode}
            cursor={hoverPos}
            isAllocated={visibleAllocated.has(hoverNode.id)}
            allocatedSameKey={allocatedSameKey}
            previewAddedCount={previewAddedCount}
            previewRemovedCount={previewRemovedCount}
          />,
          document.body,
        )}
    </div>
  )
}
