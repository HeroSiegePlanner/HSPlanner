import { useState } from "react";
import { useBuild } from "../../store/build";
import { activeSeasonId, getClass } from "@data";
import { encodeBuildToShare } from "../../utils/build/shareBuild";
import { getSavedBuild, type SavedBuild } from "../../utils/build/savedBuilds";
import { buildSharePayload, postWebShare } from "../../utils/build/webShare";
import { ShareDialog, type ShareDialogProps } from "./ShareDialog";

type ShareState = Omit<ShareDialogProps, "onClose">;

export default function ShareButton() {
  const exportSnapshot = useBuild((s) => s.exportBuildSnapshot);
  const [share, setShare] = useState<ShareState | null>(null);

  const onOpen = () => {
    if (share) {
      setShare(null);
      return;
    }
    const { notes, activeBuildId } = useBuild.getState();
    const snap = exportSnapshot();
    let loadoutsDropped = false;
    const code = encodeBuildToShare(snap, notes, activeSeasonId, {
      onLoadoutsDropped: () => {
        loadoutsDropped = true;
      },
    });
    const saved = activeBuildId ? getSavedBuild(activeBuildId) : null;
    const now = new Date().toISOString();
    const clsName = snap.classId ? getClass(snap.classId)?.name : undefined;
    const liveBuild: SavedBuild = {
      id: saved?.id ?? "live",
      name: saved?.name ?? `${clsName ?? "Hero"} Lv ${snap.level}`,
      classId: snap.classId,
      notes,
      createdAt: saved?.createdAt ?? now,
      updatedAt: now,
      profiles: [{ id: "live", name: "Current", code, updatedAt: now }],
      activeProfileId: "live",
      folderId: null,
      favorite: false,
      tags: saved?.tags ?? [],
      season: activeSeasonId,
      stash: [],
    };
    setShare({
      code,
      loadoutsDropped,
      meta: { className: snap.classId ?? undefined, level: snap.level },
      createWebShare: async () => postWebShare(await buildSharePayload(liveBuild)),
    });
  };

  return (
    <>
      <button
        onClick={onOpen}
        data-tour="share"
        className="inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep px-3 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
        style={{ background: "linear-gradient(180deg, #3a2f1a, #2a2418)" }}
        title="Share this build"
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

      {share && <ShareDialog {...share} onClose={() => setShare(null)} />}
    </>
  );
}
