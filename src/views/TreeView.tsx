import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { createPortal } from 'react-dom'
import treeBackground from '../assets/atlas/Incarnation_Background.png'
import { useBuild } from '../store/build'
import {
  ADJ,
  findPath,
  reachableFromAny,
  START_IDS,
  START_SET,
} from '../utils/tree/treeGraph'
import type { BuildPerformance } from '../utils/build/buildPerformance'
import {
  classifyTreeNodesNative,
  computeBuildPerformanceAsync,
} from '../lib/calc/bridge'
import type { NodeLineClassification } from '../lib/calc/bridge'
import { useBuildPerformanceDeps } from '../hooks/useBuildPerformanceDeps'
import { useCalcResult } from '../hooks/useCalcResult'
import JewelSocketModal from '../components/JewelSocketModal'
import SuggestNodesModal from '../components/SuggestNodesModal'
import {
  TREE_JEWELRY_IDS,
  TREE_NODE_INFO,
  TREE_WARP_IDS,
  type TreeNodeInfo,
} from '../utils/tree/treeStats'
import {
  EDGES,
  JEWELRY_NODES,
  NODE_ICONS,
  NODES,
  POS_ID_MAP,
  SEARCH_INDEX,
  vbH,
  vbW,
  vbX,
  vbY,
} from './tree/treeData'
import {
  baseStrokeWidth,
  nodeFill,
  nodeStroke,
  type NodePaint,
} from './tree/nodePaint'
import { NodeTooltip } from './tree/NodeTooltip'

export default function TreeView() {
  const allocated = useBuild((s) => s.allocatedTreeNodes)
  const toggleNode = useBuild((s) => s.toggleTreeNode)
  const resetTree = useBuild((s) => s.resetTreeNodes)
  const applySuggestedNodes = useBuild((s) => s.applySuggestedNodes)
  const treeSocketed = useBuild((s) => s.treeSocketed)
  const setTreeSocketed = useBuild((s) => s.setTreeSocketed)
  const [socketModalNodeId, setSocketModalNodeId] = useState<number | null>(null)
  const [suggestOpen, setSuggestOpen] = useState(false)
  const [suggestedPreview, setSuggestedPreview] = useState<Set<number> | null>(
    null,
  )

  const treeDeps = useBuildPerformanceDeps()

  const currentPerformance = useCalcResult<BuildPerformance | null>(
    () => computeBuildPerformanceAsync(treeDeps),
    [treeDeps],
    null,
  )

  const nodeClassifications = useCalcResult<Record<
    string,
    NodeLineClassification
  > | null>(() => classifyTreeNodesNative(), [], null)

  const [scale, setScale] = useState(0.35)
  const [tx, setTx] = useState(0)
  const [ty, setTy] = useState(0)
  const [hoverId, setHoverId] = useState<number | null>(null)
  const [hoverPos, setHoverPos] = useState<{ x: number; y: number } | null>(null)
  const [dragging, setDragging] = useState(false)
  const [viewportSize, setViewportSize] = useState({ w: 1000, h: 800 })
  const [searchQuery, setSearchQuery] = useState('')

  const containerRef = useRef<HTMLDivElement>(null)
  const dragStart = useRef<{ x: number; y: number; tx: number; ty: number } | null>(null)
  const didDragRef = useRef(false)

  const fitView = useCallback(() => {
    const el = containerRef.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    const fitW = rect.width / vbW
    const fitH = rect.height / vbH
    const fit = Math.min(fitW, fitH) * 0.95
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
      const mx = e.clientX - rect.left
      const my = e.clientY - rect.top
      const factor = e.deltaY > 0 ? 0.9 : 1.1
      const cur = scaleRef.current
      const newScale = Math.max(0.1, Math.min(5, cur * factor))
      const worldX = (mx - txRef.current) / cur
      const worldY = (my - tyRef.current) / cur
      setScale(newScale)
      setTx(mx - worldX * newScale)
      setTy(my - worldY * newScale)
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

  const hoverNode = useMemo(
    () => (hoverId != null ? NODES.find((n) => n.id === hoverId) : null),
    [hoverId],
  )

  const hoverInfo: TreeNodeInfo | null = useMemo(() => {
    if (hoverId == null) return null
    return TREE_NODE_INFO[String(hoverId)] ?? null
  }, [hoverId])

  const previewPath = useMemo(() => {
    if (hoverId == null || allocated.has(hoverId)) return null
    const sources = new Set<number>([...allocated, ...START_IDS])
    const path = findPath(sources, hoverId)
    if (!path) return null
    return new Set(path)
  }, [hoverId, allocated])

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

  const previewAllocation = useMemo<Set<number> | null>(() => {
    if (hoverId == null) return null
    if (allocated.has(hoverId)) {
      const without = new Set(allocated)
      without.delete(hoverId)
      return reachableFromAny(START_SET, without)
    }
    if (!previewPath) return null
    const next = new Set(allocated)
    for (const id of previewPath) next.add(id)
    return next
  }, [hoverId, allocated, previewPath])

  const previewPerformance = useCalcResult<BuildPerformance | null>(
    () =>
      previewAllocation
        ? computeBuildPerformanceAsync({
            ...treeDeps,
            allocatedTreeNodes: previewAllocation,
          })
        : null,
    [previewAllocation, treeDeps],
    null,
  )

  const singleNodeAllocation = useMemo<Set<number> | null>(() => {
    if (hoverId == null) return null
    if (allocated.has(hoverId)) {
      const without = new Set(allocated)
      without.delete(hoverId)
      return reachableFromAny(START_SET, without)
    }
    const next = new Set(allocated)
    next.add(hoverId)
    return next
  }, [hoverId, allocated])

  const singleNodePerformance = useCalcResult<BuildPerformance | null>(
    () =>
      singleNodeAllocation
        ? computeBuildPerformanceAsync({
            ...treeDeps,
            allocatedTreeNodes: singleNodeAllocation,
          })
        : null,
    [singleNodeAllocation, treeDeps],
    null,
  )

  const previewAddedCount = useMemo(() => {
    if (!previewAllocation) return 0
    let c = 0
    for (const id of previewAllocation) if (!allocated.has(id)) c++
    return c
  }, [allocated, previewAllocation])
  const previewRemovedCount = useMemo(() => {
    if (!previewAllocation) return 0
    let c = 0
    for (const id of allocated) if (!previewAllocation.has(id)) c++
    return c
  }, [allocated, previewAllocation])

  const searchMatches = useMemo(() => {
    const q = searchQuery.trim().toLowerCase()
    if (!q) return null
    const matches = new Set<number>()
    for (const entry of SEARCH_INDEX) {
      if (entry.haystack.includes(q)) matches.add(entry.id)
    }
    const idQuery = q.replace(/^#/, '')
    if (/^\d+$/.test(idQuery)) {
      for (const n of NODES) {
        if (String(n.id).includes(idQuery)) matches.add(n.id)
      }
    }
    return matches
  }, [searchQuery])

  const socketMarks = useMemo(
    () =>
      JEWELRY_NODES.flatMap((n) =>
        treeSocketed[n.id] != null
          ? [
              <circle
                key={`socket-mark-${n.id}`}
                cx={n.x}
                cy={n.y}
                r={n.r + 4}
                fill="none"
                stroke="#ffd66b"
                strokeWidth={2}
                opacity={0.85}
              />,
            ]
          : [],
      ),
    [treeSocketed],
  )

  const nodeCircles = useMemo(
    () =>
      NODES.map((n) => {
        const paint: NodePaint = {
          isAlloc: allocated.has(n.id),
          isPreview: false,
          isStart: START_SET.has(n.id),
          tier: n.tier,
        }
        return (
          <circle
            key={n.id}
            cx={n.x}
            cy={n.y}
            r={paint.isStart ? n.r + 3 : n.r}
            fill={nodeFill(paint)}
            stroke={nodeStroke(paint)}
            strokeWidth={baseStrokeWidth(paint.isStart, n.tier)}
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
              toggleNode(n.id)
            }}
            onContextMenu={(e) => {
              if (!TREE_JEWELRY_IDS.has(n.id) || !allocated.has(n.id)) return
              e.preventDefault()
              e.stopPropagation()
              if (didDragRef.current) {
                didDragRef.current = false
                return
              }
              setSocketModalNodeId(n.id)
            }}
          />
        )
      }),
    [allocated, toggleNode],
  )

  const previewOverlay = useMemo(() => {
    if (!previewPath) return null
    const overlayPaint: Omit<NodePaint, 'tier'> = {
      isAlloc: false,
      isPreview: true,
      isStart: false,
    }
    return NODES.filter(
      (n) => previewPath.has(n.id) && !allocated.has(n.id),
    ).map((n) => {
      const paint: NodePaint = { ...overlayPaint, tier: n.tier }
      return (
        <circle
          key={`preview-${n.id}`}
          cx={n.x}
          cy={n.y}
          r={n.r}
          fill={nodeFill(paint)}
          stroke={nodeStroke(paint)}
          strokeWidth={baseStrokeWidth(false, n.tier)}
          pointerEvents="none"
        />
      )
    })
  }, [previewPath, allocated])

  const matchOverlay = useMemo(() => {
    if (!searchMatches) return null
    return NODES.filter((n) => searchMatches.has(n.id)).map((n) => (
      <circle
        key={`match-${n.id}`}
        cx={n.x}
        cy={n.y}
        r={n.r + 4}
        fill="none"
        stroke="#e0b864"
        strokeWidth={3}
      />
    ))
  }, [searchMatches])

  const suggestedOverlay = useMemo(() => {
    if (!suggestedPreview || suggestedPreview.size === 0) return null
    return NODES.filter((n) => suggestedPreview.has(n.id)).map((n) => (
      <g key={`suggest-${n.id}`}>
        <circle
          cx={n.x}
          cy={n.y}
          r={n.r + 6}
          fill="none"
          stroke="#7fd966"
          strokeWidth={2.5}
          opacity={0.85}
          style={{ filter: 'drop-shadow(0 0 6px rgba(127,217,102,0.55))' }}
        />
        <circle
          cx={n.x}
          cy={n.y}
          r={n.r}
          fill="rgba(127,217,102,0.18)"
          stroke="#7fd966"
          strokeWidth={1.5}
          pointerEvents="none"
        />
      </g>
    ))
  }, [suggestedPreview])

  const suggestedEdgeKeys = useMemo(() => {
    if (!suggestedPreview || suggestedPreview.size === 0) return null
    const keys = new Set<string>()
    const union = new Set<number>([...allocated, ...suggestedPreview])
    for (const id of union) {
      for (const nb of ADJ.get(id) ?? []) {
        if (!union.has(nb)) continue
        if (!suggestedPreview.has(id) && !suggestedPreview.has(nb)) continue
        keys.add(id < nb ? `${id}-${nb}` : `${nb}-${id}`)
      }
    }
    return keys
  }, [suggestedPreview, allocated])

  const allocatedEdgeKeys = useMemo(() => {
    const keys = new Set<string>()
    for (const id of allocated) {
      for (const nb of ADJ.get(id) ?? []) {
        if (allocated.has(nb)) {
          keys.add(id < nb ? `${id}-${nb}` : `${nb}-${id}`)
        }
      }
    }
    return keys
  }, [allocated])

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
        style={{
          cursor: dragging ? 'grabbing' : 'grab',
        }}
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
              {EDGES.map(([x1, y1, x2, y2], i) => {
                let stroke = '#2a2f3a'
                let strokeWidth = 1.5
                const ka = `${Math.round(x1 * 10)}_${Math.round(y1 * 10)}`
                const kb = `${Math.round(x2 * 10)}_${Math.round(y2 * 10)}`
                const idA = POS_ID_MAP.get(ka)
                const idB = POS_ID_MAP.get(kb)
                if (idA != null && idB != null) {
                  if (TREE_WARP_IDS.has(idA) && TREE_WARP_IDS.has(idB)) return null
                  const key = idA < idB ? `${idA}-${idB}` : `${idB}-${idA}`
                  if (allocatedEdgeKeys.has(key)) {
                    stroke = '#c48a3a'
                    strokeWidth = 2.5
                  } else if (suggestedEdgeKeys?.has(key)) {
                    stroke = '#5fa84a'
                    strokeWidth = 2.5
                  } else if (previewEdgeKeys?.has(key)) {
                    stroke = '#8a6a2a'
                    strokeWidth = 2
                  }
                }
                return (
                  <line
                    key={i}
                    x1={x1}
                    y1={y1}
                    x2={x2}
                    y2={y2}
                    stroke={stroke}
                    strokeWidth={strokeWidth}
                  />
                )
              })}
            </g>
            <g style={{ opacity: searchMatches ? 0.25 : 1 }}>{nodeCircles}</g>
            {previewOverlay && (
              <g style={{ opacity: searchMatches ? 0.25 : 1 }}>{previewOverlay}</g>
            )}
            {suggestedOverlay && (
              <g style={{ opacity: searchMatches ? 0.35 : 1 }}>
                {suggestedOverlay}
              </g>
            )}
            <g pointerEvents="none" style={{ opacity: searchMatches ? 0.25 : 1 }}>
              {NODE_ICONS.map((icon) => {
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
            <g pointerEvents="none">{socketMarks}</g>
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

      <div className="pointer-events-none absolute right-3.5 top-3 z-10 flex items-start gap-1.5">
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
            placeholder="Search nodes or #id…"
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
              <span className="pointer-events-none absolute right-8 top-1/2 -translate-y-1/2 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot">
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
          onClick={() => setSuggestOpen(true)}
          className="pointer-events-auto rounded-[3px] border border-accent-deep bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-accent-hot transition-all hover:border-accent-hot hover:shadow-[0_0_12px_rgba(224,184,100,0.25)]"
          style={{ background: 'linear-gradient(180deg, #2a2418, #1a1410)' }}
        >
          Suggest
        </button>
        <button
          onClick={fitView}
          className="pointer-events-auto rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
        >
          Fit
        </button>
        <button
          onClick={resetTree}
          className="pointer-events-auto rounded-[3px] border border-border-2 bg-transparent px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-stat-red hover:text-stat-red"
        >
          Reset
        </button>
      </div>

      <div
        className="pointer-events-none absolute bottom-3.5 left-3.5 z-10 inline-flex items-center gap-3 rounded-[3px] border border-border px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint"
        style={{
          background:
            'linear-gradient(180deg, color-mix(in srgb, var(--color-panel-2) 80%, transparent), color-mix(in srgb, var(--color-bg) 70%, transparent))',
          backdropFilter: 'blur(6px)',
          boxShadow:
            'inset 0 1px 0 rgba(201,165,90,0.06), 0 4px 16px rgba(0,0,0,0.45)',
        }}
      >
        <span className="inline-flex items-center gap-1.5">
          <span
            aria-hidden
            className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
          />
          Nodes
          <span className="text-text">{NODES.length}</span>
        </span>
        <span aria-hidden className="h-3 w-px bg-border" />
        <span className="inline-flex items-center gap-1.5">
          <span
            aria-hidden
            className="inline-block h-1 w-1 rotate-45 bg-accent-hot"
            style={{ boxShadow: '0 0 6px rgba(224,184,100,0.6)' }}
          />
          Allocated
          <span className="text-accent-hot">{allocated.size}</span>
        </span>
        <span aria-hidden className="h-3 w-px bg-border" />
        <span className="inline-flex items-center gap-1.5">
          Zoom
          <span className="text-accent-hot">
            {(scale * 100).toFixed(0)}%
          </span>
        </span>
        {hoverNode && (
          <>
            <span aria-hidden className="h-3 w-px bg-border" />
            <span className="inline-flex items-center gap-1.5">
              Hover
              <span className="text-text">#{hoverNode.id}</span>
              <span className="text-faint">· {hoverNode.tier}</span>
            </span>
          </>
        )}
      </div>

      {hoverNode && hoverPos && !dragging && currentPerformance &&
        createPortal(
          <NodeTooltip
            key={hoverNode.id}
            node={hoverNode}
            info={hoverInfo}
            classification={
              nodeClassifications?.[String(hoverNode.id)] ?? null
            }
            cursor={hoverPos}
            socketContent={
              TREE_JEWELRY_IDS.has(hoverNode.id)
                ? treeSocketed[hoverNode.id] ?? null
                : null
            }
            isJewelry={TREE_JEWELRY_IDS.has(hoverNode.id)}
            isAllocated={allocated.has(hoverNode.id)}
            currentPerformance={currentPerformance}
            previewPerformance={previewPerformance}
            singleNodePerformance={singleNodePerformance}
            previewAddedCount={previewAddedCount}
            previewRemovedCount={previewRemovedCount}
          />,
          document.body,
        )}

      {socketModalNodeId != null && (
        <JewelSocketModal
          nodeId={socketModalNodeId}
          current={treeSocketed[socketModalNodeId] ?? null}
          onClose={() => setSocketModalNodeId(null)}
          onApply={(content) => {
            setTreeSocketed(socketModalNodeId, content)
          }}
        />
      )}

      {suggestOpen && (
        <SuggestNodesModal
          currentAllocation={allocated}
          deps={treeDeps}
          onPreviewChange={setSuggestedPreview}
          onApply={(nodes) => applySuggestedNodes(nodes)}
          onClose={() => {
            setSuggestOpen(false)
            setSuggestedPreview(null)
          }}
        />
      )}
    </div>
  )
}
