import { isTauri } from "@tauri-apps/api/core";

export type InstallPhase =
  | "idle"
  | "checking"
  | "downloading"
  | "installing"
  | "done"
  | "error";

export interface InstallProgress {
  phase: InstallPhase;
  bytesDownloaded?: number;
  bytesTotal?: number;
  error?: string;
}

export type ProgressCallback = (p: InstallProgress) => void;

export function inTauriRuntime(): boolean {
  // Returns true when the page is running inside the Tauri shell rather than a regular browser. Used to gate calls to the Tauri updater/process plugins so the same code path can fall back to opening a download URL in the web build.
  return isTauri();
}

async function runInstall(
  onProgress: ProgressCallback,
  relaunchAfter: boolean,
): Promise<void> {
  // Drives the Tauri updater plugin end-to-end: dynamically imports the updater modules, queries for an available update, downloads and installs it while reporting byte progress via onProgress, and optionally relaunches the app. Used by both installUpdate (immediate restart) and installUpdateOnQuit (deferred install).
  const { check } = await import("@tauri-apps/plugin-updater");
  const { relaunch } = await import("@tauri-apps/plugin-process");

  onProgress({ phase: "checking" });
  const update = await check();
  if (!update) {
    onProgress({ phase: "done" });
    return;
  }

  let total = 0;
  let downloaded = 0;

  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started": {
        total = event.data.contentLength ?? 0;
        onProgress({
          phase: "downloading",
          bytesDownloaded: 0,
          bytesTotal: total,
        });
        break;
      }
      case "Progress": {
        downloaded += event.data.chunkLength;
        onProgress({
          phase: "downloading",
          bytesDownloaded: downloaded,
          bytesTotal: total,
        });
        break;
      }
      case "Finished": {
        onProgress({ phase: "installing" });
        break;
      }
    }
  });

  if (relaunchAfter) {
    await relaunch();
  } else {
    onProgress({ phase: "done" });
  }
}

// Reject non-https URLs so a tampered manifest can't smuggle javascript:/data: payloads.
function isSafeUpdateUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "https:";
  } catch {
    return false;
  }
}

export async function installUpdate(
  fallbackAssetUrl: string | undefined,
  onProgress: ProgressCallback,
): Promise<void> {
  // Public entry point used by the UpdateModal "Install now" action: runs the Tauri update flow with relaunch-after-install when available, or opens the fallback browser download URL when running outside Tauri. Translates exceptions into an error-phase progress callback before re-throwing.
  if (!inTauriRuntime()) {
    if (fallbackAssetUrl && isSafeUpdateUrl(fallbackAssetUrl)) {
      window.open(fallbackAssetUrl, "_blank", "noopener,noreferrer");
    }
    onProgress({ phase: "done" });
    return;
  }
  try {
    await runInstall(onProgress, true);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    onProgress({ phase: "error", error: message });
    throw err;
  }
}

export async function installUpdateOnQuit(): Promise<void> {
  // Performs a silent updater download/install without relaunching the app. Used to apply pending updates when the user quits, so the new version is in place the next time they launch.
  if (!inTauriRuntime()) return;
  await runInstall(() => {}, false);
}
