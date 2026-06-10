import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { createPortal } from 'react-dom'
import treeBackground from '../assets/atlas/Incarnation_Background.png'
import nodeIconsMap from '../data/node-icons.json'
import treeData from '../data/hero-siege-tree.json'
import { getAffix, getGem, getRune } from '../data'
import { useBuild } from '../store/build'
import {
  ADJ,
  findPath,
  reachableFromAny,
  START_IDS,
  START_SET,
} from '../utils/tree/treeGraph'
import { formatValue, rolledAffixValue, statName } from '../utils/item/stats'
import {
  diffPerformanceDps,
  diffPerformanceStats,
  type BuildPerformance,
} from '../utils/build/buildPerformance'
import {
  classifyTreeNodesNative,
  computeBuildPerformanceAsync,
} from '../lib/calc/bridge'
import type { NodeLineClassification } from '../lib/calc/bridge'
import { useBuildPerformanceDeps } from '../hooks/useBuildPerformanceDeps'
import NetChangeRow from '../components/NetChangeRow'
import type { TreeSocketContent } from '../types'
import JewelSocketModal from '../components/JewelSocketModal'
import SuggestNodesModal from '../components/SuggestNodesModal'
import {
  TooltipHeader,
  TooltipSection,
  TooltipSectionHeader,
  TooltipText,
  UnsupportedModsList,
} from '../components/Tooltip'
import { TONE_BORDER, TONE_GLOW } from '../components/tooltip-tones'
import type { TooltipTone } from '../components/tooltip-tones'
import {
  TREE_JEWELRY_IDS,
  TREE_NODE_INFO,
  TREE_WARP_IDS,
  type TreeNodeInfo,
} from '../utils/tree/treeStats'

type RawNode = [id: number, x: number, y: number, r: number]
type RawEdge = [x1: number, y1: number, x2: number, y2: number]

interface TreeNode {
  id: number
  x: number
  y: number
  r: number
  tier: 'minor' | 'notable' | 'keystone'
}

function tierTone(nodeType: string | undefined, tier: TreeNode['tier']): TooltipTone {
  if (nodeType === 'jewelry') return 'rare'
  if (nodeType === 'warp') return 'rare'
  if (nodeType === 'root') return 'angelic'
  if (nodeType === 'big' || tier === 'keystone' || tier === 'notable') return 'rare'
  return 'neutral'
}

function tierLabel(nodeType: string | undefined, tier: TreeNode['tier']): string {
  if (nodeType === 'jewelry') return 'Jewelry Socket'
  if (nodeType === 'warp') return 'Warp Node'
  if (nodeType === 'root') return 'Starting Node'
  if (nodeType === 'big') return 'Notable'
  if (tier === 'keystone') return 'Keystone'
  return 'Minor'
}

function classifyTier(r: number): TreeNode['tier'] {
  if (r >= 12) return 'keystone'
  if (r >= 10) return 'notable'
  return 'minor'
}

const VIEW_BOX = treeData.viewBox
const NODES: TreeNode[] = (treeData.nodes as RawNode[]).map(([id, x, y, r]) => ({
  id,
  x,
  y,
  r,
  tier: classifyTier(r),
}))
const EDGES: RawEdge[] = treeData.edges as RawEdge[]

const [vbX = 0, vbY = 0, vbW = 1000, vbH = 800] = VIEW_BOX.split(' ').map(Number)

const POS_ID_MAP = (() => {
  const m = new Map<string, number>()
  for (const n of NODES) {
    m.set(`${Math.round(n.x * 10)}_${Math.round(n.y * 10)}`, n.id)
  }
  return m
})()

const SEARCH_INDEX: { id: number; haystack: string }[] = Object.entries(
  TREE_NODE_INFO,
).map(([id, info]) => ({
  id: Number(id),
  haystack: (info.t + ' ' + info.l.join(' ')).toLowerCase(),
}))

const NODE_ICON_FILES = import.meta.glob<string>(
  '../assets/atlas/nodes/*.png',
  { eager: true, query: '?url', import: 'default' },
)
const NODE_ICON_URL_BY_KEY: Record<string, string> = {}
for (const [p, url] of Object.entries(NODE_ICON_FILES)) {
  const file = p.split('/').pop() ?? ''
  const key = file.replace(/\.png$/i, '')
  NODE_ICON_URL_BY_KEY[key] = url
}

const NODE_ICON_KEY_BY_ID = nodeIconsMap as Record<string, string>
const NODE_ICONS: { id: number; x: number; y: number; r: number; href: string }[] =
  NODES.flatMap((n) => {
    const key = NODE_ICON_KEY_BY_ID[String(n.id)]
    const href = key ? NODE_ICON_URL_BY_KEY[key] : undefined
    return href ? [{ id: n.id, x: n.x, y: n.y, r: n.r, href }] : []
  })

const JEWELRY_NODES = NODES.filter((n) => TREE_JEWELRY_IDS.has(n.id))

interface NodePaint {
  isAlloc: boolean
  isHover: boolean
  isPreview: boolean
  isStart: boolean
  tier: TreeNode['tier']
}

function nodeFill({ isAlloc, isPreview, isStart, tier }: NodePaint): string {
  if (isAlloc) return tier === 'keystone' ? '#e94f37' : '#c9a55a'
  if (isPreview) return '#5a4528'
  if (isStart) return '#3a3528'
  return '#1c1d24'
}

function baseStrokeWidth(isStart: boolean, tier: TreeNode['tier']): number {
  if (isStart) return 2.5
  if (tier === 'minor') return 1
  return 2
}

function nodeStroke({
  isAlloc,
  isHover,
  isPreview,
  isStart,
  tier,
}: NodePaint): string {
  if (isAlloc) return '#d4cfbf'
  if (isHover || isPreview) return '#e0b864'
  if (isStart) return '#c9a55a'
  if (tier === 'keystone') return '#8a6f3a'
  if (tier === 'notable') return '#5a5448'
  return '#3a3528'
}

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

  const [currentPerformance, setCurrentPerformance] =
    useState<BuildPerformance | null>(null)
  useEffect(() => {
    let cancelled = false
    computeBuildPerformanceAsync(treeDeps).then((p) => {
      if (!cancelled) setCurrentPerformance(p)
    })
    return () => {
      cancelled = true
    }
  }, [treeDeps])

  const [nodeClassifications, setNodeClassifications] = useState<Record<
    string,
    NodeLineClassification
  > | null>(null)
  useEffect(() => {
    let cancelled = false
    classifyTreeNodesNative().then((m) => {
      if (!cancelled) setNodeClassifications(m)
    })
    return () => {
      cancelled = true
    }
  }, [])

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

  // React attaches onWheel as passive in v17+, so e.preventDefault() is a NO-OP
  // and the page scrolls under the tree. Attach a native non-passive listener
  // and read scale/tx/ty from refs to avoid stale closures on rapid wheels.
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

  // Mirrors `toggleTreeNode`: union with cheapest path on allocate, `reachableFromAny` cleanup on deallocate.
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

  const [previewPerformance, setPreviewPerformance] =
    useState<BuildPerformance | null>(null)
  useEffect(() => {
    if (!previewAllocation) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setPreviewPerformance(null)
      return
    }
    let cancelled = false
    computeBuildPerformanceAsync({
      ...treeDeps,
      allocatedTreeNodes: previewAllocation,
    }).then((p) => {
      if (!cancelled) setPreviewPerformance(p)
    })
    return () => {
      cancelled = true
    }
  }, [previewAllocation, treeDeps])

  // "This node alone" preview for evaluating standalone value. Clicking never produces this exact allocation.
  const singleNodeAllocation = useMemo<Set<number> | null>(() => {
    if (hoverId == null) return null
    if (allocated.has(hoverId)) {
      // Removing a bridge node strands its descendants — mirror toggleTreeNode's cleanup.
      const without = new Set(allocated)
      without.delete(hoverId)
      return reachableFromAny(START_SET, without)
    }
    const next = new Set(allocated)
    next.add(hoverId)
    return next
  }, [hoverId, allocated])

  const [singleNodePerformance, setSingleNodePerformance] =
    useState<BuildPerformance | null>(null)
  useEffect(() => {
    if (!singleNodeAllocation) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setSingleNodePerformance(null)
      return
    }
    let cancelled = false
    computeBuildPerformanceAsync({
      ...treeDeps,
      allocatedTreeNodes: singleNodeAllocation,
    }).then((p) => {
      if (!cancelled) setSingleNodePerformance(p)
    })
    return () => {
      cancelled = true
    }
  }, [singleNodeAllocation, treeDeps])

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
          isHover: false,
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
              // Right-click on allocated jewelry opens the socket picker; left-click stays as allocate/deallocate.
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
      isHover: false,
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

function NodeTooltip({
  node,
  info,
  classification,
  cursor,
  socketContent,
  isJewelry,
  isAllocated,
  currentPerformance,
  previewPerformance,
  singleNodePerformance,
  previewAddedCount,
  previewRemovedCount,
}: {
  node: TreeNode
  info: TreeNodeInfo | null
  classification: NodeLineClassification | null
  cursor: { x: number; y: number }
  socketContent: TreeSocketContent | null
  isJewelry: boolean
  isAllocated: boolean
  currentPerformance: BuildPerformance
  previewPerformance: BuildPerformance | null
  singleNodePerformance: BuildPerformance | null
  previewAddedCount: number
  previewRemovedCount: number
}) {
  const ref = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)
  const tone = tierTone(info?.n, node.tier)
  const tierName = tierLabel(info?.n, node.tier)
  const lineGroups = info ? classification : null

  const singleDpsDiffs = useMemo(
    () =>
      singleNodePerformance
        ? diffPerformanceDps(currentPerformance, singleNodePerformance)
        : [],
    [currentPerformance, singleNodePerformance],
  )
  const singleStatDiffs = useMemo(
    () =>
      singleNodePerformance
        ? diffPerformanceStats(currentPerformance, singleNodePerformance)
        : [],
    [currentPerformance, singleNodePerformance],
  )
  const pathDpsDiffs = useMemo(
    () =>
      previewPerformance
        ? diffPerformanceDps(currentPerformance, previewPerformance)
        : [],
    [currentPerformance, previewPerformance],
  )
  const pathStatDiffs = useMemo(
    () =>
      previewPerformance
        ? diffPerformanceStats(currentPerformance, previewPerformance)
        : [],
    [currentPerformance, previewPerformance],
  )
  const singleHasContent = singleDpsDiffs.length > 0 || singleStatDiffs.length > 0
  const pathHasContent = pathDpsDiffs.length > 0 || pathStatDiffs.length > 0
  const pathDiffersFromSingle =
    pathHasContent &&
    (previewAddedCount > 1 ||
      previewRemovedCount > 1 ||
      pathDpsDiffs.length !== singleDpsDiffs.length ||
      pathStatDiffs.length !== singleStatDiffs.length)
  const netChangeVisible = !isJewelry && (singleHasContent || pathHasContent)

  const [colCount, setColCount] = useState(1)
  const measuredColsRef = useRef(false)

  // Strip column/max-height to read true natural height via scrollHeight; pinned via measuredColsRef so cursor moves only re-run positioning.
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const margin = 12
    const offset = 16
    const vw = window.innerWidth
    const vh = window.innerHeight
    const heightLimit = vh - 2 * margin

    if (!measuredColsRef.current) {
      const prevColumnCount = el.style.columnCount
      const prevMaxHeight = el.style.maxHeight
      el.style.columnCount = '1'
      el.style.maxHeight = 'none'
      const naturalHeight = el.scrollHeight
      el.style.columnCount = prevColumnCount
      el.style.maxHeight = prevMaxHeight

      const needed = Math.max(
        1,
        Math.min(Math.ceil(naturalHeight / heightLimit), 5),
      )
      measuredColsRef.current = true
      if (needed !== colCount) {
        setColCount(needed)
        return
      }
    }

    const rect = el.getBoundingClientRect()
    let left = cursor.x + offset
    if (left + rect.width + margin > vw) left = cursor.x - rect.width - offset
    left = Math.max(margin, Math.min(left, vw - rect.width - margin))
    let top = cursor.y + offset
    if (top + rect.height + margin > vh) top = cursor.y - rect.height - offset
    top = Math.max(margin, Math.min(top, vh - rect.height - margin))
    setPos({ left, top })
  }, [cursor.x, cursor.y, info, node.id, colCount])

  return (
    <div
      ref={ref}
      role="tooltip"
      className={`fixed z-[1000] bg-panel border ${TONE_BORDER[tone]} ${TONE_GLOW[tone]} rounded-[4px] overflow-hidden pointer-events-none select-none shadow-[0_8px_32px_rgba(0,0,0,0.8)]`}
      style={{
        left: pos?.left ?? -9999,
        top: pos?.top ?? -9999,
        opacity: pos ? 1 : 0,
        transition: 'opacity 80ms ease-out',
        columnCount: colCount,
        columnGap: colCount > 1 ? 0 : undefined,
        columnFill: 'auto',
        columnRule:
          colCount > 1 ? '1px solid var(--color-border)' : undefined,
        width: colCount > 1 ? `${colCount * 320}px` : undefined,
        minWidth: colCount === 1 ? 240 : undefined,
        maxWidth: colCount === 1 ? 360 : 'calc(100vw - 24px)',
        maxHeight: 'calc(100vh - 24px)',
      }}
    >
      <TooltipHeader
        title={info?.t ?? `Node #${node.id}`}
        subtitle={tierName}
        tone={tone}
      />
      {isJewelry ? (
        <JewelrySocketSection
          content={socketContent}
          isAllocated={isAllocated}
        />
      ) : (
        <>
          {lineGroups && lineGroups.parsed.length > 0 && (
            <TooltipSection>
              <div className="space-y-0.5">
                {lineGroups.parsed.map((line, i) => (
                  <TooltipText key={i}>{line}</TooltipText>
                ))}
              </div>
            </TooltipSection>
          )}
          {lineGroups && lineGroups.unsupported.length > 0 && (
            <TooltipSection>
              <UnsupportedModsList lines={lineGroups.unsupported} />
            </TooltipSection>
          )}
          {info?.note && (
            <TooltipSection>
              <div className="text-[12px] leading-[1.55] text-accent-hot italic">
                {info.note}
              </div>
            </TooltipSection>
          )}
          {netChangeVisible && (
            <TooltipSection>
              <div className="mb-2 flex items-center gap-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-muted">
                <span>Net Change</span>
                <span className="h-px flex-1 bg-border" />
                {isAllocated && previewRemovedCount > 1 && (
                  <span className="font-normal tracking-[0.14em] text-faint">
                    −{previewRemovedCount} on click
                  </span>
                )}
                {!isAllocated && previewAddedCount > 1 && (
                  <span className="font-normal tracking-[0.14em] text-faint">
                    +{previewAddedCount} on click
                  </span>
                )}
              </div>
              {singleHasContent && (
                <div className={pathDiffersFromSingle ? 'mb-3' : undefined}>
                  <div className="mb-1 flex items-baseline gap-2 font-mono text-[9px] uppercase tracking-[0.16em] text-faint">
                    <span className="text-accent-deep">This Node</span>
                    {pathDiffersFromSingle && (
                      <span className="text-faint/70 normal-case">— hovered alone</span>
                    )}
                  </div>
                  {singleDpsDiffs.length > 0 && (
                    <div className="mb-1.5">
                      <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                        Active Skill
                        {currentPerformance.activeSkillName ? (
                          <span className="ml-1 text-faint">
                            · {currentPerformance.activeSkillName}
                          </span>
                        ) : null}
                      </div>
                      <div className="space-y-0.5">
                        {singleDpsDiffs.map((d) => (
                          <NetChangeRow key={d.key} diff={d} />
                        ))}
                      </div>
                    </div>
                  )}
                  {singleStatDiffs.length > 0 && (
                    <div>
                      {singleDpsDiffs.length > 0 && (
                        <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                          Stats
                        </div>
                      )}
                      <div className="space-y-0.5">
                        {singleStatDiffs.map((d) => (
                          <NetChangeRow key={d.key} diff={d} />
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
              {pathDiffersFromSingle && (
                <div>
                  <div className="mb-1 flex items-baseline gap-2 font-mono text-[9px] uppercase tracking-[0.16em] text-faint">
                    <span className="text-accent-deep">
                      {isAllocated ? 'With Cleanup' : 'Full Path'}
                    </span>
                    <span className="text-faint/70 normal-case">
                      —{' '}
                      {isAllocated
                        ? `${previewRemovedCount} nodes lost`
                        : `${previewAddedCount} nodes allocated`}
                    </span>
                  </div>
                  {pathDpsDiffs.length > 0 && (
                    <div className="mb-1.5">
                      <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                        Active Skill
                        {currentPerformance.activeSkillName ? (
                          <span className="ml-1 text-faint">
                            · {currentPerformance.activeSkillName}
                          </span>
                        ) : null}
                      </div>
                      <div className="space-y-0.5">
                        {pathDpsDiffs.map((d) => (
                          <NetChangeRow key={d.key} diff={d} />
                        ))}
                      </div>
                    </div>
                  )}
                  {pathStatDiffs.length > 0 && (
                    <div>
                      {pathDpsDiffs.length > 0 && (
                        <div className="mb-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                          Stats
                        </div>
                      )}
                      <div className="space-y-0.5">
                        {pathStatDiffs.map((d) => (
                          <NetChangeRow key={d.key} diff={d} />
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </TooltipSection>
          )}
        </>
      )}
      {info && info.g && info.g.length > 0 && (
        <TooltipSection className="bg-panel-2/40">
          <div className="flex flex-wrap gap-1">
            {info.g.map((tag, i) => (
              <span
                key={i}
                className="inline-block rounded-[2px] border border-border-2 px-1.5 py-0.5 text-[10px] uppercase tracking-[0.08em] text-muted"
              >
                {tag}
              </span>
            ))}
          </div>
        </TooltipSection>
      )}
      {!info && (
        <TooltipSection>
          <TooltipText>
            <span className="text-faint">No data available</span>
          </TooltipText>
        </TooltipSection>
      )}
    </div>
  )
}

function JewelrySocketSection({
  content,
  isAllocated,
}: {
  content: TreeSocketContent | null
  isAllocated: boolean
}) {
  if (!content) {
    return (
      <TooltipSection>
        <TooltipSectionHeader tone="gold">Socketed</TooltipSectionHeader>
        <TooltipText>
          <span className="text-faint italic">
            Empty socket{isAllocated ? ' — right-click to insert' : ''}
          </span>
        </TooltipText>
      </TooltipSection>
    )
  }

  let socketedTitle: string
  let socketedSubtitle: string | null = null
  let statLines: { key: string; value: number }[] = []

  if (content.kind === 'item') {
    const source = getGem(content.id) ?? getRune(content.id)
    if (!source) {
      return (
        <TooltipSection>
          <TooltipSectionHeader tone="gold">Socketed</TooltipSectionHeader>
          <TooltipText>
            <span className="text-stat-red">
              unknown socketable: {content.id}
            </span>
          </TooltipText>
        </TooltipSection>
      )
    }
    socketedTitle = source.name
    socketedSubtitle = `T${source.tier}`
    statLines = Object.entries(source.stats)
      .filter(([, v]) => v !== 0)
      .map(([key, value]) => ({ key, value }))
  } else {
    socketedTitle = 'Uncut Jewel'
    socketedSubtitle = `${content.affixes.length} affix${
      content.affixes.length === 1 ? '' : 'es'
    }`
    statLines = content.affixes
      .map((eq) => {
        const def = getAffix(eq.affixId)
        if (!def || !def.statKey) return null
        const value = rolledAffixValue(def, eq.roll)
        if (value === 0) return null
        return { key: def.statKey, value }
      })
      .filter((x): x is { key: string; value: number } => x !== null)
  }

  return (
    <>
      <TooltipSection>
        <TooltipSectionHeader tone="gold" trailing={socketedSubtitle}>
          Socketed
        </TooltipSectionHeader>
        <div className="text-[12px] font-medium text-accent-hot">
          {socketedTitle}
        </div>
      </TooltipSection>
      {statLines.length > 0 && (
        <TooltipSection>
          <TooltipSectionHeader tone="gold">From Sockets</TooltipSectionHeader>
          <ul className="space-y-0.5 text-[12px]">
            {statLines.map(({ key, value }) => (
              <li key={key} className="text-accent">
                {formatValue(value, key)} {statName(key)}
              </li>
            ))}
          </ul>
        </TooltipSection>
      )}
    </>
  )
}
