import { useEffect, useMemo, useState } from "react";
import {
  installUpdate,
  type InstallProgress,
} from "../utils/installUpdate";
import { readStorage, writeStorage } from "../utils/storage";
import { Modal } from "./Modal";
import {
  BUILD_CHANNEL,
  formatBytes,
  parseChangelog,
  shortSha,
  type ChangelogSection,
  type ChangelogTag,
  type UpdateInfo,
} from "../utils/version";

const SKIP_KEY = "hsplanner.update.skipped_version";
const AUTO_INSTALL_KEY = "hsplanner.update.auto_install";

const TAG_LABEL: Record<ChangelogTag, string> = {
  new: "NEW",
  improved: "IMPROVED",
  balance: "BALANCE",
  fixes: "FIXES",
  other: "NOTES",
};

const TAG_CLASS: Record<ChangelogTag, string> = {
  new: "border-stat-green/50 text-stat-green",
  improved: "border-stat-blue/50 text-stat-blue",
  balance: "border-stat-orange/50 text-stat-orange",
  fixes: "border-stat-red/50 text-stat-red",
  other: "border-border-2 text-muted",
};

const TAG_BG: Record<ChangelogTag, string> = {
  new: "linear-gradient(180deg, rgba(28,52,34,0.55), rgba(20,38,24,0.35))",
  improved:
    "linear-gradient(180deg, rgba(26,40,60,0.55), rgba(18,28,44,0.35))",
  balance:
    "linear-gradient(180deg, rgba(58,42,22,0.55), rgba(42,30,18,0.35))",
  fixes: "linear-gradient(180deg, rgba(60,30,28,0.55), rgba(44,22,20,0.35))",
  other: "var(--color-panel-2)",
};

const FOOTER_BTN_CLASS =
  "rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot disabled:cursor-not-allowed disabled:opacity-40";

const FOOTER_BTN_PRIMARY_CLASS =
  "rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4] disabled:cursor-not-allowed disabled:opacity-60";

interface Props {
  info: UpdateInfo;
  onClose: () => void;
  onSkipVersion?: (version: string) => void;
  mode?: "update" | "changelog";
}

export default function UpdateModal({
  info,
  onClose,
  onSkipVersion,
  mode = "update",
}: Props) {
  const isChangelog = mode === "changelog";
  const sections: ChangelogSection[] = useMemo(
    () => (info.body ? parseChangelog(info.body) : []),
    [info.body],
  );

  const [autoInstall, setAutoInstall] = useState<boolean>(
    () => readStorage(AUTO_INSTALL_KEY) === "1",
  );
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const isBusy =
    progress !== null &&
    progress.phase !== "done" &&
    progress.phase !== "error";

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !isBusy) onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, isBusy]);

  const onAutoInstallChange = (v: boolean) => {
    setAutoInstall(v);
    writeStorage(AUTO_INSTALL_KEY, v ? "1" : "0");
  };

  const onRemindLater = () => {
    if (!isBusy) onClose();
  };

  const onSkip = () => {
    if (isBusy) return;
    writeStorage(SKIP_KEY, info.latest);
    onSkipVersion?.(info.latest);
    onClose();
  };

  const onDownload = async () => {
    if (isBusy) return;
    setProgress({ phase: "checking" });
    try {
      await installUpdate(info.assetUrl ?? info.releaseUrl, setProgress);
      onClose();
    } catch {
      void 0;
    }
  };

  // Don't close mid-install so a backdrop click can't cancel a download.
  const safeClose = () => {
    if (!isBusy) onClose();
  };

  const releaseDateText = info.publishedAt
    ? formatReleaseDate(info.publishedAt)
    : null;

  const channelLabel = BUILD_CHANNEL === "dev" ? "DEV" : "STABLE";
  const channelTone =
    BUILD_CHANNEL === "dev"
      ? {
          color: "text-accent-hot border-accent-deep/50",
          bg: "linear-gradient(180deg, rgba(58,46,24,0.6), rgba(42,36,24,0.4))",
        }
      : {
          color: "text-stat-green border-stat-green/50",
          bg: "linear-gradient(180deg, rgba(28,52,34,0.6), rgba(20,38,24,0.4))",
        };

  return (
    <Modal
      portal={false}
      onClose={safeClose}
      closeDisabled={isBusy}
      panelClassName="max-h-[88vh] w-160 max-w-[92vw]"
      titleId="update-modal-title"
      titleClassName="truncate"
      eyebrow={
        <>
          {isChangelog ? "Changelog" : "Update"}
          {isChangelog && (
            <span
              title={
                BUILD_CHANNEL === "dev"
                  ? "Development build"
                  : "Stable build"
              }
              className={`ml-1 rounded-[3px] border px-1.5 py-px font-mono text-[9px] uppercase tracking-[0.18em] ${channelTone.color}`}
              style={{ background: channelTone.bg }}
            >
              {channelLabel}
            </span>
          )}
        </>
      }
      title={
        isChangelog ? `HSPlanner v${info.current}` : "Update Available"
      }
      subtitle={
        isChangelog
          ? "What's new in this version"
          : "A newer version of HSPlanner is ready"
      }
    >
        {!isChangelog && (
          <section
            className="grid grid-cols-[1fr_auto_1fr] items-center gap-4 border-b border-border px-5 py-4"
            style={{ background: "rgba(0,0,0,0.2)" }}
          >
            <VersionCell label="Installed" version={info.current} tone="muted" />
            <span
              aria-hidden
              className="font-mono text-[18px] tracking-[0.18em] text-accent-deep"
            >
              ▸
            </span>
            <VersionCell
              label="Available"
              version={info.latest}
              tone="hot"
              align="right"
            />
          </section>
        )}

        <section className="flex-1 overflow-y-auto px-5 py-4">
          <div className="mb-3 flex items-center gap-2 border-b border-accent-deep/20 pb-1.5">
            <span
              aria-hidden
              className="inline-block h-1 w-1 rotate-45 bg-accent-deep"
            />
            <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot/70">
              Changelog
            </span>
            {releaseDateText && (
              <>
                <span aria-hidden className="text-faint">
                  ·
                </span>
                <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
                  {releaseDateText}
                </span>
              </>
            )}
          </div>

          {sections.length === 0 ? (
            <p className="font-mono text-[12px] tracking-[0.04em] text-muted italic">
              {info.body
                ? "No structured changelog detected."
                : "No release notes available."}
            </p>
          ) : (
            <div className="space-y-4">
              {sections.map((s, i) => (
                <ChangelogBlock key={`${s.title}-${i}`} section={s} />
              ))}
            </div>
          )}
        </section>

        {!isChangelog && (
          <section
            className="flex flex-wrap items-center gap-x-5 gap-y-1 border-t border-border px-5 py-2.5 text-[11px] text-muted"
            style={{ background: "rgba(0,0,0,0.25)" }}
          >
            {info.assetSize !== undefined && (
              <MetaCell label="Size" value={formatBytes(info.assetSize)} />
            )}
            {info.assetSha && (
              <MetaCell
                label="SHA-256"
                value={shortSha(info.assetSha)}
                title={info.assetSha}
              />
            )}
            <span className="ml-auto inline-flex items-center gap-1.5">
              <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
                Channel
              </span>
              <span className="font-mono text-[11px] text-text">stable</span>
            </span>
          </section>
        )}

        {!isChangelog && progress && <ProgressBlock progress={progress} />}

        {!isChangelog && (
          <footer
            className="flex flex-wrap items-center gap-3 border-t border-border px-5 py-3"
            style={{ background: "rgba(0,0,0,0.3)" }}
          >
            <label className="flex cursor-pointer items-center gap-2 font-mono text-[10px] uppercase tracking-[0.14em] text-muted">
              <input
                type="checkbox"
                checked={autoInstall}
                onChange={(e) => onAutoInstallChange(e.target.checked)}
                disabled={isBusy}
              />
              Auto-install on quit
            </label>
            <div className="ml-auto flex items-center gap-2">
              <button
                type="button"
                onClick={onRemindLater}
                disabled={isBusy}
                className={FOOTER_BTN_CLASS}
              >
                Remind Me Later
              </button>
              <button
                type="button"
                onClick={onSkip}
                disabled={isBusy}
                className={FOOTER_BTN_CLASS}
              >
                Skip This Version
              </button>
              <button
                type="button"
                onClick={onDownload}
                disabled={isBusy}
                className={FOOTER_BTN_PRIMARY_CLASS}
                style={{
                  background: "linear-gradient(180deg, #3a2f1a, #2a2418)",
                }}
              >
                {progressLabel(progress)}
              </button>
            </div>
          </footer>
        )}
    </Modal>
  );
}

function VersionCell({
  label,
  version,
  tone,
  align = "left",
}: {
  label: string;
  version: string;
  tone: "muted" | "hot";
  align?: "left" | "right";
}) {
  return (
    <div className={align === "right" ? "text-right" : "text-left"}>
      <div className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
        {label}
      </div>
      <div
        className={`font-mono text-[20px] tabular-nums ${tone === "hot" ? "text-accent-hot" : "text-text"}`}
        style={
          tone === "hot"
            ? { textShadow: "0 0 12px rgba(224,184,100,0.25)" }
            : undefined
        }
      >
        v{version}
      </div>
    </div>
  );
}

function MetaCell({
  label,
  value,
  title,
}: {
  label: string;
  value: string;
  title?: string;
}) {
  return (
    <span className="inline-flex items-center gap-1.5" title={title}>
      <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
        {label}
      </span>
      <span className="font-mono text-[11px] text-text">{value}</span>
    </span>
  );
}

function ChangelogBlock({ section }: { section: ChangelogSection }) {
  const label = TAG_LABEL[section.tag];
  const cls = TAG_CLASS[section.tag];
  const bg = TAG_BG[section.tag];
  return (
    <div>
      <div
        className={`mb-2 inline-block rounded-[3px] border px-2 py-0.5 font-mono text-[10px] font-semibold uppercase tracking-[0.18em] ${cls}`}
        style={{ background: bg }}
      >
        {label}
      </div>
      <ul className="space-y-1.5 text-[13px] text-text">
        {section.items.map((it, i) => (
          <li key={i} className="flex gap-2.5 leading-relaxed">
            <span
              aria-hidden
              className="mt-2 inline-block h-1 w-1 shrink-0 rotate-45 bg-accent-deep"
            />
            <span dangerouslySetInnerHTML={{ __html: renderInline(it) }} />
          </li>
        ))}
      </ul>
    </div>
  );
}

// Escape HTML first to defang injection, then apply a tiny markdown subset.
function renderInline(text: string): string {
  const escaped = text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  return escaped
    .replace(/\*\*([^*]+)\*\*/g, '<strong class="text-accent-hot">$1</strong>')
    .replace(
      /`([^`]+)`/g,
      '<code class="font-mono text-[12px] rounded-xs bg-panel-2 px-1 text-accent-hot">$1</code>',
    );
}

function formatReleaseDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d
    .toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    })
    .toUpperCase();
}

function progressLabel(p: InstallProgress | null): string {
  if (!p) return "↓ Download & Install";
  switch (p.phase) {
    case "checking":
      return "Checking…";
    case "downloading": {
      if (p.bytesTotal && p.bytesTotal > 0 && p.bytesDownloaded !== undefined) {
        const pct = Math.floor((p.bytesDownloaded / p.bytesTotal) * 100);
        return `Downloading ${pct}%`;
      }
      return "Downloading…";
    }
    case "installing":
      return "Installing…";
    case "done":
      return "Done";
    case "error":
      return "Retry";
    default:
      return "↓ Download & Install";
  }
}

function ProgressBlock({ progress }: { progress: InstallProgress }) {
  const pct =
    progress.phase === "downloading" &&
    progress.bytesTotal &&
    progress.bytesTotal > 0 &&
    progress.bytesDownloaded !== undefined
      ? Math.min(100, (progress.bytesDownloaded / progress.bytesTotal) * 100)
      : progress.phase === "installing" || progress.phase === "done"
        ? 100
        : null;
  const bytes =
    progress.bytesDownloaded !== undefined && progress.bytesTotal
      ? `${formatBytes(progress.bytesDownloaded)} / ${formatBytes(progress.bytesTotal)}`
      : null;
  const isError = progress.phase === "error";
  return (
    <section
      className="border-t border-border px-5 py-3 text-[11px]"
      style={{ background: "rgba(0,0,0,0.3)" }}
    >
      <div className="mb-1.5 flex items-center justify-between">
        <span
          className={`inline-flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.18em] ${isError ? "text-stat-red" : "text-accent-hot/80"}`}
        >
          <span
            aria-hidden
            className={`inline-block h-1 w-1 rotate-45 ${isError ? "bg-stat-red" : "bg-accent-hot animate-pulse"}`}
            style={
              isError
                ? { boxShadow: "0 0 6px rgba(217,107,90,0.6)" }
                : { boxShadow: "0 0 6px rgba(224,184,100,0.6)" }
            }
          />
          {isError ? "Error" : phaseLabel(progress.phase)}
        </span>
        {bytes && (
          <span className="font-mono text-[11px] tabular-nums text-faint">
            {bytes}
          </span>
        )}
      </div>
      {isError ? (
        <p className="font-mono text-[11px] text-stat-red">
          {progress.error ?? "Install failed"}
        </p>
      ) : (
        <div
          className="h-1.5 w-full overflow-hidden rounded-full border border-border"
          style={{ background: "var(--color-panel)" }}
        >
          <div
            className="h-full transition-all"
            style={{
              width: pct !== null ? `${pct}%` : "30%",
              background:
                "linear-gradient(90deg, var(--color-accent-deep), var(--color-accent-hot))",
              boxShadow: "0 0 8px rgba(224,184,100,0.45)",
              animation:
                pct === null
                  ? "pulse 1.4s cubic-bezier(0.4, 0, 0.6, 1) infinite"
                  : undefined,
            }}
          />
        </div>
      )}
    </section>
  );
}

function phaseLabel(phase: InstallProgress["phase"]): string {
  switch (phase) {
    case "checking":
      return "Checking for update";
    case "downloading":
      return "Downloading";
    case "installing":
      return "Installing";
    case "done":
      return "Done";
    default:
      return "";
  }
}

