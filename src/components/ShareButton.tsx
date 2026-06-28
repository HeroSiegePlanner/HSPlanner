import { useEffect, useState } from "react";
import { useBuild } from "../store/build";
import { encodeBuildToShare } from "../utils/build/shareBuild";
import { Modal } from "./Modal";
import {
  GistShareError,
  isGistSharingConfigured,
  uploadBuildToGist,
} from "../utils/build/gistShare";

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
  const [meta, setMeta] = useState<{ className?: string; level?: number }>({});

  const generate = (): string => {
    const { notes } = useBuild.getState();
    const snap = exportSnapshot();
    const next = encodeBuildToShare(snap, notes);
    setCode(next);
    setMeta({ className: snap.classId ?? undefined, level: snap.level });
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
          meta={meta}
          status={status}
          onClose={() => setOpen(false)}
          onCopy={onCopy}
        />
      )}
    </>
  );
}

export function ShareDialog({
  code,
  meta,
  status,
  onClose,
  onCopy,
}: {
  code: string;
  meta?: { className?: string; level?: number };
  status: Status;
  onClose: () => void;
  onCopy: () => void;
}) {
  type GistState =
    | { kind: "idle" }
    | { kind: "uploading" }
    | { kind: "done"; url: string }
    | { kind: "error"; message: string };
  const [gist, setGist] = useState<GistState>({ kind: "idle" });
  const [linkCopied, setLinkCopied] = useState(false);
  const gistConfigured = isGistSharingConfigured();

  const onShareGist = async () => {
    setGist({ kind: "uploading" });
    try {
      const { url } = await uploadBuildToGist(code, meta);
      setGist({ kind: "done", url });
    } catch (e) {
      const message =
        e instanceof GistShareError ? e.message : "Gist upload failed.";
      setGist({ kind: "error", message });
    }
  };

  const onCopyLink = async (url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      setLinkCopied(true);
      setTimeout(() => setLinkCopied(false), 2500);
    } catch {
      /* ignore */
    }
  };

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

  return (
    <Modal
      onClose={onClose}
      eyebrow="Share"
      title="Build Code"
      panelClassName="max-h-[88vh] w-[34rem] max-w-[94vw]"
    >
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

          <div className="mt-1 flex flex-col gap-2 border-t border-border pt-3">
            <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
              Or share a link
            </span>
            {gist.kind === "done" ? (
              <div className="flex items-center gap-2">
                <input
                  readOnly
                  value={gist.url}
                  onFocus={(e) => e.currentTarget.select()}
                  className="min-w-0 flex-1 rounded-[3px] border border-border-2 px-3 py-2 font-mono text-[11px] text-text focus:border-accent-deep focus:outline-none"
                  style={{
                    background:
                      "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
                  }}
                />
                <button
                  onClick={() => onCopyLink(gist.url)}
                  className={FOOTER_BTN_PRIMARY_CLASS}
                  style={{ background: "linear-gradient(180deg, #3a2f1a, #2a2418)" }}
                >
                  {linkCopied ? "Copied" : "Copy link"}
                </button>
              </div>
            ) : (
              <button
                onClick={onShareGist}
                disabled={!gistConfigured || gist.kind === "uploading"}
                title={
                  gistConfigured
                    ? "Upload this build to a GitHub Gist"
                    : "Gist sharing is not configured in this build"
                }
                className={`${FOOTER_BTN_PRIMARY_CLASS} self-start disabled:cursor-not-allowed disabled:opacity-50`}
                style={{ background: "linear-gradient(180deg, #3a2f1a, #2a2418)" }}
              >
                {gist.kind === "uploading" ? "Uploading…" : "Share via Gist"}
              </button>
            )}
            {gist.kind === "error" && (
              <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-stat-red">
                {gist.message}
              </span>
            )}
            {!gistConfigured && (
              <span className="font-mono text-[10px] tracking-[0.04em] text-faint">
                Gist sharing is not configured in this build.
              </span>
            )}
          </div>
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
    </Modal>
  );
}

