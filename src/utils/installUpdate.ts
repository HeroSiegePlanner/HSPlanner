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
  return isTauri();
}

async function runInstall(
  onProgress: ProgressCallback,
  relaunchAfter: boolean,
): Promise<void> {
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
  if (!inTauriRuntime()) return;
  await runInstall(() => {}, false);
}
