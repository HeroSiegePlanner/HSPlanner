import { useMemo } from 'react'
import treeBackground from '../assets/atlas/Incarnation_Background.png'
import { useBuild } from '../store/build'
import { ADJ } from '../utils/tree/treeGraph'
import {
  ALL_TREE_EDGES,
  ALL_TREE_NODES,
  type TreeNodeEntry,
} from '../utils/treeNodes'

const ZOOM_VB_W = 600
const ZOOM_VB_H = 340
const ZOOM_MARGIN = 60

const EDGE_DEFAULT = '#2a2f3a'
const EDGE_ALLOCATED = '#c48a3a'

interface Props {
  node: TreeNodeEntry
  width?: number
}

export default function TreeNodeMiniMap({ node, width = 340 }: Props) {
  const allocated = useBuild((s) => s.allocatedTreeNodes)
  const view = useMemo(() => {
    const vbX = node.x - ZOOM_VB_W / 2
    const vbY = node.y - ZOOM_VB_H / 2
    const xMin = vbX - ZOOM_MARGIN
    const xMax = vbX + ZOOM_VB_W + ZOOM_MARGIN
    const yMin = vbY - ZOOM_MARGIN
    const yMax = vbY + ZOOM_VB_H + ZOOM_MARGIN
    const nodes = ALL_TREE_NODES.filter(
      (n) => n.x >= xMin && n.x <= xMax && n.y >= yMin && n.y <= yMax,
    )
    const edges = ALL_TREE_EDGES.filter(([x1, y1, x2, y2]) => {
      const aIn = x1 >= xMin && x1 <= xMax && y1 >= yMin && y1 <= yMax
      const bIn = x2 >= xMin && x2 <= xMax && y2 >= yMin && y2 <= yMax
      return aIn || bIn
    })
    return { vbX, vbY, nodes, edges }
  }, [node.x, node.y])
  const allocatedEdgeKeys = useMemo(() => {
    const keys = new Set<string>()
    for (const a of allocated) {
      const nbrs = ADJ.get(a)
      if (!nbrs) continue
      for (const b of nbrs) {
        if (!allocated.has(b)) continue
        keys.add(a < b ? `${a}-${b}` : `${b}-${a}`)
      }
    }
    return keys
  }, [allocated])
  const POS_TO_ID = useMemo(() => {
    const m = new Map<string, number>()
    for (const n of ALL_TREE_NODES) {
      m.set(`${Math.round(n.x * 10)}_${Math.round(n.y * 10)}`, n.id)
    }
    return m
  }, [])
  return (
    <div
      className="rounded-[4px] overflow-hidden"
      style={{
        width,
        background:
          'linear-gradient(180deg, var(--color-panel-2), var(--color-bg))',
      }}
    >
      <div
        className="border-b border-accent-deep/30 px-3 py-1.5"
        style={{
          background:
            'linear-gradient(180deg, rgba(224,184,100,0.14), rgba(224,184,100,0.04))',
        }}
      >
        <div className="font-mono text-[9px] uppercase tracking-[0.14em] text-faint">
          Tree node
        </div>
        <div className="mt-0.5 text-[12px] font-semibold text-accent-hot leading-tight">
          {node.name}
        </div>
        <div className="font-mono text-[9px] uppercase tracking-[0.12em] text-text/40 mt-0.5">
          #{node.id} · {node.kind || 'normal'}
        </div>
      </div>
      <div
        style={{
          backgroundColor: '#0a0b0f',
          backgroundImage: `url(${treeBackground})`,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          aspectRatio: `${ZOOM_VB_W} / ${ZOOM_VB_H}`,
        }}
      >
        <svg
          viewBox={`${view.vbX} ${view.vbY} ${ZOOM_VB_W} ${ZOOM_VB_H}`}
          className="block w-full h-full"
          preserveAspectRatio="xMidYMid meet"
        >
          <g>
            {view.edges.map(([x1, y1, x2, y2], i) => {
              const ka = `${Math.round(x1 * 10)}_${Math.round(y1 * 10)}`
              const kb = `${Math.round(x2 * 10)}_${Math.round(y2 * 10)}`
              const idA = POS_TO_ID.get(ka)
              const idB = POS_TO_ID.get(kb)
              let stroke = EDGE_DEFAULT
              let strokeWidth = 1.5
              if (idA != null && idB != null) {
                const key = idA < idB ? `${idA}-${idB}` : `${idB}-${idA}`
                if (allocatedEdgeKeys.has(key)) {
                  stroke = EDGE_ALLOCATED
                  strokeWidth = 2.5
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
          <g>
            {view.nodes.map((n) => {
              const size = n.r * 2.4
              const isAlloc = allocated.has(n.id)
              if (n.iconUrl) {
                return (
                  <g key={n.id}>
                    {isAlloc && (
                      <circle
                        cx={n.x}
                        cy={n.y}
                        r={n.r * 1.4}
                        fill="none"
                        stroke="#c9a55a"
                        strokeWidth={1.5}
                      />
                    )}
                    <image
                      href={n.iconUrl}
                      x={n.x - size / 2}
                      y={n.y - size / 2}
                      width={size}
                      height={size}
                      preserveAspectRatio="xMidYMid meet"
                      style={{ imageRendering: 'pixelated' }}
                    />
                  </g>
                )
              }
              return (
                <circle
                  key={n.id}
                  cx={n.x}
                  cy={n.y}
                  r={n.r * 0.8}
                  fill={
                    isAlloc
                      ? '#c9a55a'
                      : n.kind === 'root'
                        ? 'rgba(220, 184, 100, 0.55)'
                        : n.kind === 'jewelry'
                          ? 'rgba(110, 180, 220, 0.55)'
                          : 'rgba(160, 160, 160, 0.55)'
                  }
                />
              )
            })}
          </g>
          <g pointerEvents="none">
            <circle
              cx={node.x}
              cy={node.y}
              r={Math.max(node.r * 2.2, 18)}
              fill="none"
              stroke="#e94f37"
              strokeWidth={3}
              style={{
                filter: 'drop-shadow(0 0 6px rgba(233,79,55,0.85))',
              }}
            />
          </g>
        </svg>
      </div>
    </div>
  )
}
