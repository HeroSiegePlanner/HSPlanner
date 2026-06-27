import { getSavedBuild } from "../utils/build/savedBuilds";
import { useBuild } from "../store/build";

const HEADER_BTN_CLASS =
  "inline-flex items-center gap-1.5 rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot";

interface BuildsMenuProps {
  onOpenLibrary: () => void;
}

export default function BuildsMenu({ onOpenLibrary }: BuildsMenuProps) {
  const activeBuildId = useBuild((s) => s.activeBuildId);
  // Subscribe so the active build name refreshes when saved builds change.
  useBuild((s) => s.savedBuildsVersion);
  const activeBuild = activeBuildId ? getSavedBuild(activeBuildId) : null;

  return (
    <button
      onClick={onOpenLibrary}
      className={HEADER_BTN_CLASS}
      title="Build library"
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
        <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" />
      </svg>
      Builds
      {activeBuild && (
        <>
          <span aria-hidden className="text-faint">
            ·
          </span>
          <span className="max-w-[8rem] truncate text-accent-hot">
            {activeBuild.name}
          </span>
        </>
      )}
    </button>
  );
}
