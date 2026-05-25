import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useBuild } from "../store/build";
import { encodeBuildToShare } from "../utils/shareBuild";

type Status = "idle" | "copied" | "error";

const FOOTER_BTN_CLASS =
  "rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot";

const FOOTER_BTN_PRIMARY_CLASS =
  "rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]";

export default function ShareButton() {
  const exportSnapshot = useBuild((s) => s.exportBuildSnapshot);
  const [status, setStatus] = useState<Status>("idle");
  const [code, setCode] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  const generate = (): string => {
    const { notes } = useBuild.getState();
    const next = encodeBuildToShare(exportSnapshot(), notes);
    setCode(next);
    setOpen(true);
    setStatus("idle");
    return next;
  };

  const onOpen = () => {
    if (open) {
      setOpen(false);
      return;
    }
    generate();
  };

  const onCopy = async () => {
    const next = code ?? generate();
    try {
      await navigator.clipboard.writeText(next);
      setStatus("copied");
      setTimeout(() => setStatus("idle"), 2500);
    } catch {
      setStatus("error");
      setTimeout(() => setStatus("idle"), 2500);
    }
  };

  return (
    <>
      <button
        onClick={onOpen}
        className="inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep px-3 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
        style={{ background: "linear-gradient(180deg, #3a2f1a, #2a2418)" }}
        title="Generate shareable build code"
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
        >
          <circle cx="18" cy="5" r="3" />
          <circle cx="6" cy="12" r="3" />
          <circle cx="18" cy="19" r="3" />
          <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
          <line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
        </svg>
        Share
      </button>

      {open && code && (
        <ShareDialog
          code={code}
          status={status}
          onClose={() => setOpen(false)}
          onCopy={onCopy}
        />
      )}
    </>
  );
}

function ShareDialog({
  code,
  status,
  onClose,
  onCopy,
}: {
  code: string;
  status: Status;
  onClose: () => void;
  onCopy: () => void;
}) {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const copyLabel =
    status === "copied"
      ? "Copied"
      : status === "error"
        ? "Copy failed"
        : "Copy code";

  const statusText =
    status === "copied"
      ? "Code copied to clipboard"
      : status === "error"
        ? "Clipboard write failed"
        : `${code.length} characters`;

  const statusColor =
    status === "copied"
      ? "text-accent-hot"
      : status === "error"
        ? "text-stat-red"
        : "text-faint";

  return createPortal(
    <div
      role="presentation"
      className="fixed inset-0 z-100 flex items-center justify-center backdrop-blur-sm"
      onMouseDown={onClose}
      style={{
        background:
          "radial-gradient(ellipse at 50% 0%, rgba(201,165,90,0.06), rgba(0,0,0,0.78) 60%)",
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
        className="relative flex max-h-[88vh] w-[34rem] max-w-[94vw] flex-col overflow-hidden rounded-[6px] border border-border"
        style={{
          background:
            "linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))",
          boxShadow:
            "inset 0 1px 0 rgba(255,255,255,0.02), 0 24px 64px rgba(0,0,0,0.7)",
        }}
      >
        <CornerMarks />
        <header
          className="flex items-start justify-between gap-3 border-b border-border px-5 py-4"
          style={{
            background:
              "linear-gradient(180deg, rgba(201,165,90,0.05), transparent)",
          }}
        >
          <div>
            <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              <span
                aria-hidden
                className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
                style={{ boxShadow: "0 0 8px rgba(224,184,100,0.6)" }}
              />
              Share
            </div>
            <h2
              className="m-0 text-[18px] font-semibold tracking-[0.02em] text-accent-hot"
              style={{ textShadow: "0 0 16px rgba(224,184,100,0.15)" }}
            >
              Build Code
            </h2>
          </div>
          <button
            onClick={onClose}
            className="rounded-[3px] border border-border-2 bg-panel-2 px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Close
          </button>
        </header>

        <div className="flex flex-col gap-3 p-5">
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            Read-only build payload
          </span>
          <textarea
            value={code}
            readOnly
            onFocus={(e) => e.currentTarget.select()}
            rows={6}
            className="w-full rounded-[3px] border border-border-2 px-3 py-2 font-mono text-[11px] tabular-nums text-text focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
            style={{
              background:
                "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
              boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
            }}
          />
          <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            Paste into Builds → Import to load on another device.
          </p>
        </div>

        <footer className="flex items-center justify-between gap-3 border-t border-border bg-black/30 px-4 py-3">
          <div
            className={`flex min-w-0 flex-1 items-center gap-2 font-mono text-[11px] tracking-[0.06em] ${statusColor}`}
          >
            <span
              aria-hidden
              className="h-1.5 w-1.5 shrink-0 rounded-full"
              style={{
                background:
                  status === "copied"
                    ? "var(--color-accent-hot)"
                    : status === "error"
                      ? "var(--color-stat-red)"
                      : "var(--color-faint)",
                boxShadow:
                  status === "copied"
                    ? "0 0 8px rgba(224,184,100,0.6)"
                    : status === "error"
                      ? "0 0 8px rgba(217,107,90,0.6)"
                      : "0 0 6px var(--color-faint)",
              }}
            />
            <span className="truncate">{statusText}</span>
          </div>
          <div className="flex shrink-0 gap-2">
            <button
              onClick={onCopy}
              className={FOOTER_BTN_PRIMARY_CLASS}
              style={{
                background: "linear-gradient(180deg, #3a2f1a, #2a2418)",
              }}
            >
              {copyLabel}
            </button>
            <button onClick={onClose} className={FOOTER_BTN_CLASS}>
              Close
            </button>
          </div>
        </footer>
      </div>
    </div>,
    document.body,
  );
}

function CornerMarks() {
  const base: React.CSSProperties = {
    position: "absolute",
    width: 10,
    height: 10,
    border: "1px solid var(--color-accent-deep)",
    opacity: 0.55,
    pointerEvents: "none",
  };
  return (
    <>
      <span
        style={{
          ...base,
          top: -1,
          left: -1,
          borderRight: "none",
          borderBottom: "none",
        }}
      />
      <span
        style={{
          ...base,
          top: -1,
          right: -1,
          borderLeft: "none",
          borderBottom: "none",
        }}
      />
      <span
        style={{
          ...base,
          bottom: -1,
          left: -1,
          borderRight: "none",
          borderTop: "none",
        }}
      />
      <span
        style={{
          ...base,
          bottom: -1,
          right: -1,
          borderLeft: "none",
          borderTop: "none",
        }}
      />
    </>
  );
}
