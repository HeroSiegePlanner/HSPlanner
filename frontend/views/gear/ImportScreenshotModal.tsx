import { useCallback, useEffect, useRef, useState } from 'react'
import { getItem } from '@data'
import type { EquippedItem, ItemBase } from '../../types'
import {
  MODAL_BTN_CLASS,
  MODAL_BTN_PRIMARY_CLASS,
  MODAL_FOOTER_CLASS,
  Modal,
} from '../../components/ui/Modal'
import { useBuild } from '../../store/build'
import { ocrTooltipImage } from '../../utils/item/ocr'
import {
  parseTooltipLines,
  type TooltipParseResult,
} from '../../utils/item/tooltipParse'

interface ImportScreenshotModalProps {
  onClose: () => void
  onEquip: (item: EquippedItem, base: ItemBase) => string | null
  ocr?: (blob: Blob) => Promise<string[]>
}

const STATUS_CLASS = {
  matched: 'text-text',
  warning: 'text-accent-hot',
  ignored: 'text-faint',
} as const

export default function ImportScreenshotModal({
  onClose,
  onEquip,
  ocr = ocrTooltipImage,
}: ImportScreenshotModalProps) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [result, setResult] = useState<TooltipParseResult | null>(null)
  const [imageUrl, setImageUrl] = useState<string | null>(null)
  const fileRef = useRef<HTMLInputElement>(null)

  const handleImage = useCallback(
    async (blob: Blob) => {
      setBusy(true)
      setError(null)
      setResult(null)
      setImageUrl((prev) => {
        if (prev) URL.revokeObjectURL(prev)
        return URL.createObjectURL(blob)
      })
      try {
        const lines = await ocr(blob)
        setResult(parseTooltipLines(lines))
      } catch (err: unknown) {
        setError(
          err instanceof Error
            ? err.message
            : String(err) ||
                'OCR unavailable — run the desktop app to import screenshots',
        )
      } finally {
        setBusy(false)
      }
    },
    [ocr],
  )

  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      const item = [...(e.clipboardData?.items ?? [])].find((i) =>
        i.type.startsWith('image/'),
      )
      const blob = item?.getAsFile()
      if (blob) {
        e.preventDefault()
        void handleImage(blob)
      }
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
  }, [handleImage])

  useEffect(
    () => () => {
      if (imageUrl) URL.revokeObjectURL(imageUrl)
    },
    [imageUrl],
  )

  const base = result?.baseId ? getItem(result.baseId) : undefined
  const equipped = result?.equipped ?? null
  const warnings = result?.lines.filter((l) => l.status === 'warning') ?? []

  const handleEquip = () => {
    if (!equipped || !base) return
    const err = onEquip(equipped, base)
    if (err) {
      setError(err)
      return
    }
    onClose()
  }

  const handleStash = () => {
    if (!equipped) return
    useBuild.getState().addStashItem(equipped)
    onClose()
  }

  return (
    <Modal
      onClose={onClose}
      panelClassName="w-[560px] max-w-[94vw]"
      eyebrow="Import"
      title="Import from screenshot"
      subtitle={
        base
          ? `${base.name} — ${base.rarity}`
          : 'Paste (Ctrl+V) a tooltip screenshot or choose a file'
      }
    >
      <div className="flex max-h-[60vh] flex-col gap-3 overflow-y-auto px-6 py-4">
        <div className="flex items-center gap-3">
          <button
            type="button"
            className={MODAL_BTN_CLASS}
            onClick={() => fileRef.current?.click()}
            disabled={busy}
          >
            {busy ? 'Reading…' : 'Choose image'}
          </button>
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            or paste with Ctrl+V
          </span>
          <input
            ref={fileRef}
            type="file"
            accept="image/*"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0]
              if (f) void handleImage(f)
              e.target.value = ''
            }}
          />
        </div>

        {imageUrl && (
          <img
            src={imageUrl}
            alt="Tooltip screenshot"
            className="max-h-40 self-start rounded-[3px] border border-border object-contain"
          />
        )}

        {error && (
          <p className="m-0 font-mono text-[11px] text-red-400">{error}</p>
        )}
        {result?.errors.map((e) => (
          <p key={e} className="m-0 font-mono text-[11px] text-red-400">
            {e}
          </p>
        ))}

        {result && equipped && (
          <>
            {warnings.length > 0 && (
              <section>
                <h3 className="m-0 mb-1 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot">
                  Manual review ({warnings.length})
                </h3>
                <ul className="m-0 flex list-none flex-col gap-0.5 p-0 font-mono text-[11px]">
                  {warnings.map((line, i) => (
                    <li
                      key={`${i}-${line.text}`}
                      className={STATUS_CLASS.warning}
                      title={line.detail}
                    >
                      ⚠ {line.text}
                      {line.detail && (
                        <span className="text-faint"> — {line.detail}</span>
                      )}
                    </li>
                  ))}
                </ul>
              </section>
            )}
            <details>
              <summary className="cursor-pointer font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-faint hover:text-muted">
                Debug ({result.lines.length} lines)
              </summary>
              <ul className="m-0 mt-1 flex list-none flex-col gap-0.5 p-0 font-mono text-[11px]">
                {result.lines.map((line, i) => (
                  <li
                    key={`${i}-${line.text}`}
                    className={STATUS_CLASS[line.status]}
                    title={line.detail}
                  >
                    {line.status === 'warning' ? '⚠ ' : ''}
                    {line.text}
                    {line.detail && line.status !== 'ignored' && (
                      <span className="text-faint"> — {line.detail}</span>
                    )}
                  </li>
                ))}
              </ul>
            </details>
          </>
        )}
      </div>

      <div className={MODAL_FOOTER_CLASS}>
        <button
          type="button"
          className={MODAL_BTN_CLASS}
          onClick={handleStash}
          disabled={!equipped}
        >
          Add to stash
        </button>
        <button
          type="button"
          className={MODAL_BTN_PRIMARY_CLASS}
          onClick={handleEquip}
          disabled={!equipped || !base}
        >
          {base ? `Equip (${base.slot})` : 'Equip'}
        </button>
      </div>
    </Modal>
  )
}
