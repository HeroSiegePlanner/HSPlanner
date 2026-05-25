import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { motion } from "motion/react";
import { backdropVariants, panelVariants } from "../lib/motion";
import { classes, getClass, getClassIcon } from "../data";
import { useBuild } from "../store/build";
import {
  listSavedBuilds,
  type SavedBuild,
  type SavedProfile,
} from "../utils/savedBuilds";
import {
  decodeShareToBuild,
  parseBuildCodeFromInput,
} from "../utils/shareBuild";

type SortKey = "lastOpened" | "name" | "level";

type Scope =
  | { kind: "recent" }
  | { kind: "all" }
  | { kind: "class"; classId: string };

type Mode =
  | "menu"
  | "save"
  | "import"
  | "renameBuild"
  | "renameProfile"
  | "addProfile";

interface BuildMeta {
  level: number;
  nodes: number;
  classId: string | null;
  className: string;
  decoded: boolean;
}

interface ContextMenuState {
  x: number;
  y: number;
  kind: "build" | "profile";
  buildId: string;
  profileId?: string;
}

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

const RECENT_LIMIT = 12;

export default function StartupBuildModal({ isOpen, onClose }: Props) {
  const activeBuildId = useBuild((s) => s.activeBuildId);
  const activeProfileId = useBuild((s) => s.activeProfileId);
  const importBuildSnapshot = useBuild((s) => s.importBuildSnapshot);
  const loadSavedBuildAction = useBuild((s) => s.loadSavedBuild);
  const saveCurrentAsNewBuild = useBuild((s) => s.saveCurrentAsNewBuild);
  const deleteSavedBuild = useBuild((s) => s.deleteSavedBuild);
  const renameSavedBuildAction = useBuild((s) => s.renameSavedBuild);
  const commitActiveProfile = useBuild((s) => s.commitActiveProfile);
  const switchActiveProfile = useBuild((s) => s.switchActiveProfile);
  const addProfileToActiveBuild = useBuild(
    (s) => s.addProfileToActiveBuild,
  );
  const duplicateActiveProfile = useBuild((s) => s.duplicateActiveProfile);
  const renameActiveProfile = useBuild((s) => s.renameActiveProfile);
  const removeActiveProfile = useBuild((s) => s.removeActiveProfile);
  const savedBuildsVersion = useBuild((s) => s.savedBuildsVersion);

  const builds = useMemo(
    () => (isOpen ? listSavedBuilds() : []),
    // savedBuildsVersion is the store cache-buster: re-read on every store mutation.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [isOpen, savedBuildsVersion],
  );

  const metaByBuild = useMemo<Record<string, BuildMeta>>(() => {
    const out: Record<string, BuildMeta> = {};
    for (const b of builds) {
      const profile =
        b.profiles.find((p) => p.id === b.activeProfileId) ?? b.profiles[0];
      const cls = b.classId ? getClass(b.classId) : undefined;
      let level = 1;
      let nodes = 0;
      let decoded = false;
      if (profile) {
        const share = decodeShareToBuild(profile.code);
        if (share) {
          level = share.snapshot.level;
          nodes = share.snapshot.allocatedTreeNodes.size;
          decoded = true;
        }
      }
      out[b.id] = {
        level,
        nodes,
        classId: b.classId,
        className: cls?.name ?? "Unknown",
        decoded,
      };
    }
    return out;
  }, [builds]);

  const classCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const b of builds) {
      if (!b.classId) continue;
      counts[b.classId] = (counts[b.classId] ?? 0) + 1;
    }
    return counts;
  }, [builds]);

  const availableClasses = useMemo(
    () => classes.filter((c) => (classCounts[c.id] ?? 0) > 0),
    [classCounts],
  );

  const [mode, setMode] = useState<Mode>("menu");
  const [scope, setScope] = useState<Scope>({ kind: "recent" });
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<SortKey>("lastOpened");
  const [preferredId, setPreferredId] = useState<string | null>(null);
  const [saveName, setSaveName] = useState("");
  const [importText, setImportText] = useState("");
  const [importError, setImportError] = useState<string | null>(null);
  const [renameBuildTarget, setRenameBuildTarget] =
    useState<SavedBuild | null>(null);
  const [renameProfileTarget, setRenameProfileTarget] =
    useState<SavedProfile | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [addProfileValue, setAddProfileValue] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [pendingDeleteKey, setPendingDeleteKey] = useState<string | null>(
    null,
  );
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(
    null,
  );

    const fileInputRef = useRef<HTMLInputElement | null>(null);

  // Reset transient UI on close so the next open is clean.
  useEffect(() => {
    if (isOpen) return;
    // One-shot reset keyed to close transition, not a render loop.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setMode("menu");
    setSearch("");
    setContextMenu(null);
    setPendingDeleteKey(null);
    setImportText("");
    setImportError(null);
    setSaveName("");
    setAddProfileValue("");
    setRenameBuildTarget(null);
    setRenameProfileTarget(null);
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    // One-shot init keyed to open transition, not a render loop.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setPreferredId((cur) => cur ?? activeBuildId ?? builds[0]?.id ?? null);
    setScope((cur) => {
      if (builds.length === 0) return { kind: "all" };
      return cur;
    });
  }, [isOpen, activeBuildId, builds]);

  useEffect(() => {
    if (!notice) return;
    const t = window.setTimeout(() => setNotice(null), 2000);
    return () => window.clearTimeout(t);
  }, [notice]);

  const flash = (msg: string) => {
    setNotice(msg);
  };

  const visible = useMemo(() => {
    const filter = search.trim().toLowerCase();
    let list = builds.slice();

    if (scope.kind === "class") {
      list = list.filter((b) => b.classId === scope.classId);
    }

    if (filter) {
      list = list.filter((b) => {
        if (b.name.toLowerCase().includes(filter)) return true;
        const meta = metaByBuild[b.id];
        if (meta && meta.className.toLowerCase().includes(filter)) return true;
        if (b.profiles.some((p) => p.name.toLowerCase().includes(filter)))
          return true;
        return false;
      });
    }

    list.sort((a, b) => {
      if (sort === "name") return a.name.localeCompare(b.name);
      if (sort === "level") {
        const la = metaByBuild[a.id]?.level ?? 0;
        const lb = metaByBuild[b.id]?.level ?? 0;
        return lb - la;
      }
      return (
        new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
      );
    });

    if (scope.kind === "recent") list = list.slice(0, RECENT_LIMIT);
    return list;
  }, [builds, metaByBuild, scope, search, sort]);

  const selectedId = useMemo(() => {
    if (preferredId && visible.some((b) => b.id === preferredId))
      return preferredId;
    return visible[0]?.id ?? null;
  }, [visible, preferredId]);

  const selectedBuild = selectedId
    ? builds.find((b) => b.id === selectedId)
    : null;
  const selectedMeta = selectedBuild ? metaByBuild[selectedBuild.id] : null;

  const doLoad = (buildId: string) => {
    if (buildId === activeBuildId) {
      onClose();
      return;
    }
    const ok = loadSavedBuildAction(buildId);
    if (!ok) {
      flash("Failed to load build");
      return;
    }
    const target = builds.find((b) => b.id === buildId);
    flash(`Loaded "${target?.name ?? "build"}"`);
    onClose();
  };

  const doSaveAsNew = () => {
    const name = saveName.trim() || "Untitled build";
    const notes = useBuild.getState().notes;
    const rec = saveCurrentAsNewBuild(name, notes);
    // Null means write rejected (e.g. localStorage full); keep dialog open since StorageErrorBanner explains.
    if (!rec) return;
    flash(`Saved "${rec.name}"`);
    setSaveName("");
    setMode("menu");
  };

  const doOverwriteActive = () => {
    if (!activeBuildId || !activeProfileId) return;
    if (commitActiveProfile()) flash("Updated active profile");
  };

  const tryImport = (text: string) => {
    const code = parseBuildCodeFromInput(text);
    if (!code) {
      setImportError("Couldn't read a build code from input");
      return;
    }
    const decoded = decodeShareToBuild(code);
    if (!decoded) {
      setImportError("Invalid or corrupted build code");
      return;
    }
    setImportError(null);
    importBuildSnapshot(decoded.snapshot, decoded.notes);
    setImportText("");
    setMode("menu");
    flash("Build imported");
    onClose();
  };

  const handleFile = (file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      const text = String(reader.result ?? "");
      tryImport(text);
    };
    reader.onerror = () => setImportError("Could not read file");
    reader.readAsText(file);
  };

  const doStartRenameBuild = (b: SavedBuild) => {
    setRenameBuildTarget(b);
    setRenameValue(b.name);
    setMode("renameBuild");
  };

  const doRenameBuild = () => {
    if (!renameBuildTarget) return;
    renameSavedBuildAction(
      renameBuildTarget.id,
      renameValue.trim() || renameBuildTarget.name,
    );
    setRenameBuildTarget(null);
    setMode("menu");
  };

  const doStartRenameProfile = (p: SavedProfile) => {
    setRenameProfileTarget(p);
    setRenameValue(p.name);
    setMode("renameProfile");
  };

  const doRenameProfile = () => {
    if (!renameProfileTarget) return;
    renameActiveProfile(
      renameProfileTarget.id,
      renameValue.trim() || renameProfileTarget.name,
    );
    setRenameProfileTarget(null);
    setMode("menu");
  };

  const doDeleteBuild = (b: SavedBuild) => {
    const key = `build:${b.id}`;
    if (pendingDeleteKey !== key) {
      setPendingDeleteKey(key);
      return;
    }
    deleteSavedBuild(b.id);
    setPendingDeleteKey(null);
    flash(`Deleted "${b.name}"`);
  };

  const doActivateProfile = (p: SavedProfile) => {
    if (p.id === activeProfileId) return;
    if (switchActiveProfile(p.id)) flash(`Profile: ${p.name}`);
  };

  const doDuplicateProfile = (p: SavedProfile) => {
    if (duplicateActiveProfile(p.id)) flash(`Duplicated "${p.name}"`);
  };

  const doAddProfile = () => {
    const activeBuild = builds.find((b) => b.id === activeBuildId);
    const fallback = activeBuild
      ? `Profile ${activeBuild.profiles.length + 1}`
      : "Profile";
    const name = addProfileValue.trim() || fallback;
    if (addProfileToActiveBuild(name)) {
      flash(`Added profile "${name}"`);
      setAddProfileValue("");
      setMode("menu");
    }
  };

  const doDeleteProfile = (p: SavedProfile) => {
    const activeBuild = builds.find((b) => b.id === activeBuildId);
    if (!activeBuild) return;
    if (activeBuild.profiles.length <= 1) {
      flash("Cannot remove the last profile");
      return;
    }
    const key = `profile:${p.id}`;
    if (pendingDeleteKey !== key) {
      setPendingDeleteKey(key);
      return;
    }
    if (removeActiveProfile(p.id)) {
      setPendingDeleteKey(null);
      flash(`Removed profile "${p.name}"`);
    }
  };

  const openContextMenu = (
    e: React.MouseEvent,
    payload: Omit<ContextMenuState, "x" | "y">,
  ) => {
    e.preventDefault();
    e.stopPropagation();
    setPendingDeleteKey(null);
    setContextMenu({ x: e.clientX, y: e.clientY, ...payload });
  };

  useEffect(() => {
    if (!isOpen) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        if (mode !== "menu") {
          setMode("menu");
          return;
        }
        if (contextMenu) {
          setContextMenu(null);
          return;
        }
        onClose();
      }
      if (
        e.key === "Enter" &&
        mode === "menu" &&
        !contextMenu &&
        selectedId
      ) {
        const tag = (e.target as HTMLElement | null)?.tagName;
        if (tag !== "TEXTAREA" && tag !== "INPUT") doLoad(selectedId);
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, mode, contextMenu, selectedId, onClose]);

  if (!isOpen) return null;

  return createPortal(
    <motion.div
      role="presentation"
      className="fixed inset-0 z-150 flex items-center justify-center backdrop-blur-sm"
      onMouseDown={onClose}
      variants={backdropVariants}
      initial="initial"
      animate="animate"
      style={{
        background:
          "radial-gradient(ellipse at 50% 0%, rgba(201,165,90,0.08), rgba(0,0,0,0.85) 65%)",
      }}
    >
      <motion.div
        role="dialog"
        aria-modal="true"
        aria-label="Select build"
        onMouseDown={(e) => e.stopPropagation()}
        variants={panelVariants}
        initial="initial"
        animate="animate"
        className="relative flex max-h-[90vh] w-230 max-w-[94vw] flex-col overflow-hidden rounded-md border border-border"
        style={{
          background:
            "linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))",
          boxShadow:
            "inset 0 1px 0 rgba(255,255,255,0.02), 0 28px 72px rgba(0,0,0,0.72)",
        }}
      >
        <CornerMarks />

        <header
          className="flex items-start justify-between gap-3 border-b border-border px-6 py-5"
          style={{
            background:
              "linear-gradient(180deg, rgba(201,165,90,0.06), transparent)",
          }}
        >
          <div>
            <div className="mb-1 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.22em] text-faint">
              <span
                aria-hidden
                className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot"
                style={{ boxShadow: "0 0 10px rgba(224,184,100,0.7)" }}
              />
              Library
              <span className="text-accent-hot">{builds.length}</span>
            </div>
            <h2
              className="m-0 text-[20px] font-semibold uppercase tracking-[0.16em] text-accent-hot"
              style={{ textShadow: "0 0 18px rgba(224,184,100,0.18)" }}
            >
              Select Build
            </h2>
            <p className="mt-1 font-mono text-[10px] uppercase tracking-[0.18em] text-muted">
              Right-click a build or profile for more actions
            </p>
          </div>
          <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
            <button
              type="button"
              onClick={() => setMode("save")}
              className="inline-flex items-center gap-1.5 rounded-[3px] border border-border-2 bg-panel-2 px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.16em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              <SaveIcon />
              Save current
            </button>
            <button
              type="button"
              onClick={() => {
                setImportError(null);
                setMode("import");
              }}
              className="inline-flex items-center gap-1.5 rounded-[3px] border border-border-2 bg-panel-2 px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.16em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              <UploadIcon />
              Import
            </button>
            <button
              type="button"
              onClick={onClose}
              className="inline-flex items-center gap-1.5 rounded-[3px] border border-accent-deep bg-panel-2 px-3.5 py-1.5 font-mono text-[10px] uppercase tracking-[0.16em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
              style={{
                background: "linear-gradient(180deg, #3a2f1a, #2a2418)",
              }}
            >
              <PlusIcon />
              New build
            </button>
          </div>
        </header>

        <div
          className="grid min-h-112 flex-1 overflow-hidden"
          style={{ gridTemplateColumns: "220px 1fr" }}
        >
          <aside
            className="flex flex-col gap-5 overflow-y-auto border-r border-border bg-black/25 px-3 py-4"
            style={{
              backgroundImage:
                "linear-gradient(180deg, rgba(201,165,90,0.03), transparent 30%)",
            }}
          >
            <SidebarSection label="Library">
              <SidebarRow
                label="Recent"
                count={Math.min(builds.length, RECENT_LIMIT)}
                active={scope.kind === "recent"}
                onClick={() => setScope({ kind: "recent" })}
              />
              <SidebarRow
                label="All builds"
                count={builds.length}
                active={scope.kind === "all"}
                onClick={() => setScope({ kind: "all" })}
              />
            </SidebarSection>

            {availableClasses.length > 0 && (
              <SidebarSection label="By class">
                {availableClasses.map((c) => {
                  const active =
                    scope.kind === "class" && scope.classId === c.id;
                  return (
                    <SidebarRow
                      key={c.id}
                      label={c.name}
                      count={classCounts[c.id] ?? 0}
                      active={active}
                      dotColor={classColor(c.id)}
                      iconUrl={getClassIcon(c.id)}
                      onClick={() =>
                        setScope(
                          active
                            ? { kind: "all" }
                            : { kind: "class", classId: c.id },
                        )
                      }
                    />
                  );
                })}
              </SidebarSection>
            )}
          </aside>

          <section className="flex min-w-0 flex-col">
            <div className="flex items-center gap-3 border-b border-border px-4 py-3">
              <div className="relative min-w-0 flex-1">
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
                  autoFocus
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Search by name, class, or profile…"
                  className="w-full rounded-[3px] border border-border-2 px-3 py-2 pl-9 text-[13px] text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
                  style={{
                    background:
                      "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
                    boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
                  }}
                />
              </div>
              <label className="flex shrink-0 items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
                Sort
                <select
                  value={sort}
                  onChange={(e) => setSort(e.target.value as SortKey)}
                  className="rounded-[3px] border border-border-2 bg-panel-2 px-2 py-1.5 font-mono text-[11px] text-text focus:border-accent-deep focus:outline-none"
                >
                  <option value="lastOpened">Last opened</option>
                  <option value="name">Name</option>
                  <option value="level">Level</option>
                </select>
              </label>
            </div>

            <div
              className="min-h-0 flex-1 overflow-y-auto"
              onClick={() => setContextMenu(null)}
            >
              {builds.length === 0 ? (
                <EmptyState onNewBuild={onClose} />
              ) : visible.length === 0 ? (
                <div className="p-10 text-center font-mono text-[11px] uppercase tracking-[0.18em] text-faint">
                  No matches
                </div>
              ) : (
                <ul className="flex flex-col gap-1.5 p-3">
                  {visible.map((b) => {
                    const meta = metaByBuild[b.id];
                    const isActive = b.id === activeBuildId;
                    const armed = pendingDeleteKey === `build:${b.id}`;
                    return (
                      <BuildCard
                        key={b.id}
                        build={b}
                        meta={meta}
                        selected={b.id === selectedId}
                        isActive={isActive}
                        armed={armed}
                        activeProfileId={activeProfileId}
                        armedProfileKey={pendingDeleteKey}
                        onSelect={() => setPreferredId(b.id)}
                        onOpen={() => doLoad(b.id)}
                        onContextMenu={(e) =>
                          openContextMenu(e, {
                            kind: "build",
                            buildId: b.id,
                          })
                        }
                        onActivateProfile={doActivateProfile}
                        onAddProfile={() => setMode("addProfile")}
                        onProfileContextMenu={(e, p) =>
                          openContextMenu(e, {
                            kind: "profile",
                            buildId: b.id,
                            profileId: p.id,
                          })
                        }
                      />
                    );
                  })}
                </ul>
              )}
            </div>
          </section>
        </div>

        <footer className="flex items-center justify-between gap-3 border-t border-border bg-black/35 px-5 py-3">
          <div className="min-w-0 flex-1 truncate font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            {notice ? (
              <span className="text-accent-hot">{notice}</span>
            ) : selectedBuild ? (
              <>
                <span className="text-accent-hot">{selectedBuild.name}</span>
                <span className="mx-2 text-faint">·</span>
                {selectedMeta?.className ?? "—"}
                <span className="mx-2 text-faint">·</span>
                Lv {selectedMeta?.level ?? "—"}
                <span className="mx-2 text-faint">·</span>
                {selectedBuild.profiles.length}P
              </>
            ) : (
              "No build selected"
            )}
          </div>
          <div className="flex shrink-0 gap-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded-[3px] border border-border-2 bg-transparent px-4 py-1.5 font-mono text-[11px] font-semibold uppercase tracking-[0.16em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              Cancel
            </button>
            <button
              type="button"
              disabled={!selectedId}
              onClick={() => selectedId && doLoad(selectedId)}
              className="inline-flex items-center gap-2 rounded-[3px] border border-accent-deep px-4 py-1.5 font-mono text-[11px] font-semibold uppercase tracking-[0.16em] text-accent-hot transition-all hover:border-accent-hot hover:text-[#fff0c4] disabled:cursor-not-allowed disabled:border-border-2 disabled:bg-transparent disabled:text-faint disabled:shadow-none"
              style={{
                background: selectedId
                  ? "linear-gradient(180deg, #3a2f1a, #2a2418)"
                  : undefined,
                boxShadow: selectedId
                  ? "0 0 18px rgba(224,184,100,0.18)"
                  : undefined,
              }}
            >
              Open build
              <span aria-hidden>→</span>
            </button>
          </div>
        </footer>
      </motion.div>

      {mode === "import" && (
        <ImportOverlay
          text={importText}
          error={importError}
          onTextChange={setImportText}
          onClose={() => {
            setMode("menu");
            setImportText("");
            setImportError(null);
          }}
          onPickFile={() => fileInputRef.current?.click()}
          onSubmit={() => tryImport(importText)}
        />
      )}

      {mode === "save" && (
        <SaveOverlay
          name={saveName}
          activeBuildId={activeBuildId}
          activeProfileId={activeProfileId}
          onNameChange={setSaveName}
          onClose={() => setMode("menu")}
          onSaveAsNew={doSaveAsNew}
          onOverwrite={() => {
            doOverwriteActive();
            setMode("menu");
          }}
        />
      )}

      {mode === "addProfile" && (
        <MiniInputOverlay
          title="Add profile"
          sectionLabel="New profile"
          label="Profile name"
          value={addProfileValue}
          placeholder="e.g. Variant"
          submitLabel="Add"
          hint="Seeded with the current state, then activated."
          onValueChange={setAddProfileValue}
          onClose={() => setMode("menu")}
          onSubmit={doAddProfile}
        />
      )}

      {mode === "renameBuild" && renameBuildTarget && (
        <MiniInputOverlay
          title={`Rename "${renameBuildTarget.name}"`}
          sectionLabel="Rename build"
          label="New name"
          value={renameValue}
          submitLabel="Save"
          onValueChange={setRenameValue}
          onClose={() => setMode("menu")}
          onSubmit={doRenameBuild}
        />
      )}

      {mode === "renameProfile" && renameProfileTarget && (
        <MiniInputOverlay
          title={`Rename "${renameProfileTarget.name}"`}
          sectionLabel="Rename profile"
          label="New profile name"
          value={renameValue}
          submitLabel="Save"
          onValueChange={setRenameValue}
          onClose={() => setMode("menu")}
          onSubmit={doRenameProfile}
        />
      )}

      {contextMenu &&
        (() => {
          const build = builds.find((b) => b.id === contextMenu.buildId);
          if (!build) return null;
          const profile =
            contextMenu.kind === "profile" && contextMenu.profileId
              ? (build.profiles.find((p) => p.id === contextMenu.profileId) ??
                null)
              : null;
          return (
            <ContextMenu
              state={contextMenu}
              build={build}
              profile={profile}
              isActiveBuild={build.id === activeBuildId}
              isActiveProfile={profile?.id === activeProfileId}
              armedBuild={pendingDeleteKey === `build:${build.id}`}
              armedProfile={
                profile != null && pendingDeleteKey === `profile:${profile.id}`
              }
              isLastProfile={build.profiles.length <= 1}
              onClose={() => setContextMenu(null)}
              onLoadBuild={() => {
                doLoad(build.id);
                setContextMenu(null);
              }}
              onUpdateBuild={() => {
                doOverwriteActive();
                setContextMenu(null);
              }}
              onRenameBuild={() => {
                setContextMenu(null);
                doStartRenameBuild(build);
              }}
              onDeleteBuild={() => doDeleteBuild(build)}
              onActivateProfile={() => {
                if (profile) doActivateProfile(profile);
                setContextMenu(null);
              }}
              onRenameProfile={() => {
                setContextMenu(null);
                if (profile) doStartRenameProfile(profile);
              }}
              onDuplicateProfile={() => {
                if (profile) doDuplicateProfile(profile);
                setContextMenu(null);
              }}
              onDeleteProfile={() => {
                if (profile) doDeleteProfile(profile);
              }}
            />
          );
        })()}

      <input
        ref={fileInputRef}
        type="file"
        accept=".txt,.hsbuild,.hspb,application/json,text/plain"
        className="hidden"
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) handleFile(f);
          e.target.value = "";
        }}
      />
    </motion.div>,
    document.body,
  );
}

function SidebarSection({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1">
      <div className="px-2 pb-1 font-mono text-[9px] uppercase tracking-[0.22em] text-faint">
        {label}
      </div>
      <div className="flex flex-col">{children}</div>
    </div>
  );
}

function SidebarRow({
  label,
  count,
  active,
  dotColor,
  iconUrl,
  onClick,
}: {
  label: string;
  count: number;
  active: boolean;
  dotColor?: string;
  iconUrl?: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`group relative flex items-center justify-between gap-3 rounded-[3px] px-2.5 py-1.5 text-left text-[12px] transition-colors ${
        active
          ? "bg-accent-hot/10 text-accent-hot"
          : "text-muted hover:bg-white/2.5 hover:text-text"
      }`}
    >
      <span
        aria-hidden
        className={`absolute left-0 top-1.5 bottom-1.5 w-0.5 transition-opacity ${
          active ? "bg-accent-hot opacity-100" : "opacity-0"
        }`}
        style={
          active ? { boxShadow: "0 0 10px rgba(224,184,100,0.55)" } : undefined
        }
      />
      <span className="flex min-w-0 items-center gap-2">
        {iconUrl ? (
          <img
            src={iconUrl}
            alt=""
            aria-hidden
            className="inline-block h-4 w-4 shrink-0 object-contain"
            style={{
              filter: active
                ? "drop-shadow(0 0 6px rgba(224,184,100,0.45))"
                : undefined,
            }}
          />
        ) : (
          dotColor && (
            <span
              aria-hidden
              className="inline-block h-2 w-2 rounded-full"
              style={{
                backgroundColor: dotColor,
                boxShadow: active ? `0 0 8px ${dotColor}` : undefined,
              }}
            />
          )
        )}
        <span className="truncate">{label}</span>
      </span>
      <span
        className={`font-mono text-[10px] tabular-nums tracking-[0.06em] ${
          active ? "text-accent-hot" : "text-faint"
        }`}
      >
        {count}
      </span>
    </button>
  );
}

function BuildCard({
  build,
  meta,
  selected,
  isActive,
  armed,
  activeProfileId,
  armedProfileKey,
  onSelect,
  onOpen,
  onContextMenu,
  onActivateProfile,
  onAddProfile,
  onProfileContextMenu,
}: {
  build: SavedBuild;
  meta: BuildMeta | undefined;
  selected: boolean;
  isActive: boolean;
  armed: boolean;
  activeProfileId: string | null;
  armedProfileKey: string | null;
  onSelect: () => void;
  onOpen: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onActivateProfile: (p: SavedProfile) => void;
  onAddProfile: () => void;
  onProfileContextMenu: (e: React.MouseEvent, p: SavedProfile) => void;
}) {
  const color = build.classId ? classColor(build.classId) : "#5a5448";
  const initial = (meta?.className?.[0] ?? "?").toUpperCase();
  const iconUrl = build.classId ? getClassIcon(build.classId) : undefined;
  return (
    <li>
      <div
        className={`group relative flex flex-col rounded-[3px] border transition-all ${
          selected
            ? "border-accent-deep bg-accent-hot/6"
            : "border-border-2/60 bg-panel/40 hover:border-accent-deep/50 hover:bg-accent-hot/3"
        }`}
        style={{
          boxShadow: selected
            ? "0 0 0 1px rgba(224,184,100,0.18), 0 0 24px rgba(224,184,100,0.08)"
            : undefined,
        }}
      >
        <button
          type="button"
          onClick={onSelect}
          onDoubleClick={onOpen}
          onContextMenu={onContextMenu}
          className="grid w-full items-center gap-4 px-3 py-2.5 text-left"
          style={{ gridTemplateColumns: "44px 1fr auto" }}
        >
          {iconUrl ? (
            <img
              src={iconUrl}
              alt=""
              aria-hidden
              className="h-11 w-11 object-contain"
              style={{
                filter: selected
                  ? `drop-shadow(0 0 10px ${color}aa)`
                  : undefined,
              }}
            />
          ) : (
            <span
              aria-hidden
              className="flex h-11 w-11 items-center justify-center rounded-[3px] border font-mono text-[15px] font-bold tracking-tight"
              style={{
                color,
                borderColor: `${color}55`,
                background: `linear-gradient(180deg, ${color}1a, ${color}05)`,
                boxShadow: selected ? `0 0 14px ${color}40` : undefined,
              }}
            >
              {initial}
            </span>
          )}

          <div className="flex min-w-0 flex-col gap-0.5">
            <div className="flex min-w-0 items-center gap-2">
              <span
                className={`truncate text-[14px] font-medium tracking-[0.01em] ${
                  selected
                    ? "text-accent-hot"
                    : "text-text group-hover:text-accent-hot"
                }`}
              >
                {build.name}
              </span>
              {isActive && (
                <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-accent-deep">
                  Active
                </span>
              )}
              {armed && (
                <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-stat-red">
                  Click delete again
                </span>
              )}
            </div>
            <div className="flex min-w-0 items-center gap-2 font-mono text-[10px] uppercase tracking-[0.14em] text-muted">
              <span className="truncate">{meta?.className ?? "—"}</span>
              <span aria-hidden className="text-faint">
                ·
              </span>
              <span className="tabular-nums">
                {build.profiles.length} profile
                {build.profiles.length === 1 ? "" : "s"}
              </span>
              {!meta?.decoded && (
                <>
                  <span aria-hidden className="text-faint">
                    ·
                  </span>
                  <span className="text-stat-red/80">unreadable</span>
                </>
              )}
            </div>
          </div>

          <div className="flex flex-col items-end gap-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            <span className="text-[13px] tabular-nums tracking-[0.02em] text-text">
              Lv <span className="text-accent-hot">{meta?.level ?? "—"}</span>
            </span>
            <span className="tabular-nums">
              {(meta?.nodes ?? 0).toLocaleString()} nodes
            </span>
            <span>{formatTimestamp(build.updatedAt)}</span>
          </div>
        </button>

        {isActive && build.profiles.length > 0 && (
          <ProfileSubList
            profiles={build.profiles}
            activeProfileId={activeProfileId}
            armedProfileKey={armedProfileKey}
            onActivate={onActivateProfile}
            onAdd={onAddProfile}
            onContextMenu={onProfileContextMenu}
          />
        )}
      </div>
    </li>
  );
}

function ProfileSubList({
  profiles,
  activeProfileId,
  armedProfileKey,
  onActivate,
  onAdd,
  onContextMenu,
}: {
  profiles: SavedProfile[];
  activeProfileId: string | null;
  armedProfileKey: string | null;
  onActivate: (p: SavedProfile) => void;
  onAdd: () => void;
  onContextMenu: (e: React.MouseEvent, p: SavedProfile) => void;
}) {
  return (
    <div
      className="border-t border-border/60 bg-black/20"
      style={{
        backgroundImage:
          "linear-gradient(90deg, rgba(201,165,90,0.04), transparent 60%)",
      }}
    >
      <ul className="flex flex-col">
        {profiles.map((p) => {
          const active = p.id === activeProfileId;
          const armed = armedProfileKey === `profile:${p.id}`;
          return (
            <li key={p.id}>
              <button
                type="button"
                onClick={() => onActivate(p)}
                onContextMenu={(e) => onContextMenu(e, p)}
                className="group relative grid w-full items-center gap-3 px-4 py-1.5 pl-14 text-left transition-colors hover:bg-accent-hot/5"
                style={{ gridTemplateColumns: "12px 1fr auto" }}
              >
                <span
                  aria-hidden
                  className={`inline-block h-1 w-1 rotate-45 ${active ? "bg-accent-hot" : "bg-faint"}`}
                  style={
                    active
                      ? { boxShadow: "0 0 6px rgba(224,184,100,0.6)" }
                      : undefined
                  }
                />
                <span
                  className={`truncate text-[12px] ${active ? "text-accent-hot" : "text-muted group-hover:text-text"}`}
                >
                  {p.name}
                </span>
                <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
                  {active ? "Active" : armed ? "Click delete again" : ""}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
      <button
        type="button"
        onClick={onAdd}
        className="flex w-full items-center gap-2 border-t border-dashed border-border/60 px-4 py-1.5 pl-14 text-left font-mono text-[10px] uppercase tracking-[0.14em] text-faint transition-colors hover:bg-accent-hot/5 hover:text-accent-hot"
      >
        <span aria-hidden>+</span>
        Add profile
      </button>
    </div>
  );
}

function EmptyState({ onNewBuild }: { onNewBuild: () => void }) {
  return (
    <div className="flex h-full min-h-80 flex-col items-center justify-center gap-4 px-6 text-center">
      <div
        aria-hidden
        className="flex h-14 w-14 items-center justify-center rounded-full border border-accent-deep/50"
        style={{
          background:
            "radial-gradient(circle at 50% 40%, rgba(224,184,100,0.18), transparent 70%)",
          boxShadow: "0 0 28px rgba(224,184,100,0.18)",
        }}
      >
        <span
          aria-hidden
          className="h-2 w-2 rotate-45 bg-accent-hot"
          style={{ boxShadow: "0 0 10px rgba(224,184,100,0.8)" }}
        />
      </div>
      <div>
        <h3 className="m-0 text-[15px] font-semibold uppercase tracking-[0.18em] text-accent-hot">
          No saved builds
        </h3>
        <p className="mt-1 font-mono text-[11px] uppercase tracking-[0.16em] text-muted">
          Start fresh or import an existing one
        </p>
      </div>
      <button
        type="button"
        onClick={onNewBuild}
        className="inline-flex items-center gap-2 rounded-[3px] border border-accent-deep px-4 py-2 font-mono text-[11px] font-semibold uppercase tracking-[0.16em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
        style={{ background: "linear-gradient(180deg, #3a2f1a, #2a2418)" }}
      >
        <PlusIcon />
        New build
      </button>
    </div>
  );
}

function ImportOverlay({
  text,
  error,
  onTextChange,
  onClose,
  onPickFile,
  onSubmit,
}: {
  text: string;
  error: string | null;
  onTextChange: (s: string) => void;
  onClose: () => void;
  onPickFile: () => void;
  onSubmit: () => void;
}) {
  return (
    <div
      role="presentation"
      className="fixed inset-0 z-160 flex items-center justify-center backdrop-blur-[2px]"
      onMouseDown={onClose}
      style={{ background: "rgba(0,0,0,0.55)" }}
    >
      <div
        role="dialog"
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
        className="relative flex w-130 max-w-[92vw] flex-col overflow-hidden rounded-md border border-border"
        style={{
          background:
            "linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))",
          boxShadow: "0 24px 64px rgba(0,0,0,0.7)",
        }}
      >
        <CornerMarks />
        <header
          className="flex items-center justify-between gap-3 border-b border-border px-5 py-4"
          style={{
            background:
              "linear-gradient(180deg, rgba(201,165,90,0.05), transparent)",
          }}
        >
          <div>
            <div className="mb-1 font-mono text-[10px] uppercase tracking-[0.2em] text-faint">
              Import
            </div>
            <h3 className="m-0 text-[16px] font-semibold uppercase tracking-[0.14em] text-accent-hot">
              Import build
            </h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Close
          </button>
        </header>
        <div className="flex flex-col gap-3 p-5">
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            Paste a build code, share URL — or load from file
          </span>
          <textarea
            autoFocus
            value={text}
            onChange={(e) => onTextChange(e.target.value)}
            placeholder="Paste shared build code…"
            rows={6}
            className="w-full rounded-[3px] border border-border-2 px-3 py-2 font-mono text-[11px] tabular-nums text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
            style={{
              background:
                "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
              boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
            }}
          />
          <button
            type="button"
            onClick={onPickFile}
            className="self-start rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Choose file…
          </button>
          {error && (
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-stat-red">
              {error}
            </span>
          )}
        </div>
        <footer className="flex items-center justify-end gap-2 border-t border-border bg-black/30 px-5 py-3">
          <button
            type="button"
            onClick={onClose}
            className="rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onSubmit}
            className="rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
            style={{ background: "linear-gradient(180deg, #3a2f1a, #2a2418)" }}
          >
            Import & load
          </button>
        </footer>
      </div>
    </div>
  );
}

function SaveOverlay({
  name,
  activeBuildId,
  activeProfileId,
  onNameChange,
  onClose,
  onSaveAsNew,
  onOverwrite,
}: {
  name: string;
  activeBuildId: string | null;
  activeProfileId: string | null;
  onNameChange: (s: string) => void;
  onClose: () => void;
  onSaveAsNew: () => void;
  onOverwrite: () => void;
}) {
  const canOverwrite = !!activeBuildId && !!activeProfileId;
  return (
    <div
      role="presentation"
      className="fixed inset-0 z-160 flex items-center justify-center backdrop-blur-[2px]"
      onMouseDown={onClose}
      style={{ background: "rgba(0,0,0,0.55)" }}
    >
      <div
        role="dialog"
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
        className="relative flex w-115 max-w-[92vw] flex-col overflow-hidden rounded-md border border-border"
        style={{
          background:
            "linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))",
          boxShadow: "0 24px 64px rgba(0,0,0,0.7)",
        }}
      >
        <CornerMarks />
        <header
          className="flex items-center justify-between gap-3 border-b border-border px-5 py-4"
          style={{
            background:
              "linear-gradient(180deg, rgba(201,165,90,0.05), transparent)",
          }}
        >
          <div>
            <div className="mb-1 font-mono text-[10px] uppercase tracking-[0.2em] text-faint">
              Save
            </div>
            <h3 className="m-0 text-[16px] font-semibold uppercase tracking-[0.14em] text-accent-hot">
              Save build
            </h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Close
          </button>
        </header>
        <div className="flex flex-col gap-3 p-5">
          {canOverwrite && (
            <button
              type="button"
              onClick={onOverwrite}
              className="w-full justify-center rounded-[3px] border border-accent-deep px-3.5 py-2 font-mono text-[11px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
              style={{
                background: "linear-gradient(180deg, #3a2f1a, #2a2418)",
              }}
            >
              Update active profile
            </button>
          )}
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            New build name
          </span>
          <input
            autoFocus
            value={name}
            onChange={(e) => onNameChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") onSaveAsNew();
              if (e.key === "Escape") onClose();
            }}
            placeholder="e.g. Lightning Viking"
            className="w-full rounded-[3px] border border-border-2 px-3 py-2 text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
            style={{
              background:
                "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
              boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
            }}
          />
          <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
            Creates a new build with one profile seeded from current state.
          </p>
        </div>
        <footer className="flex items-center justify-end gap-2 border-t border-border bg-black/30 px-5 py-3">
          <button
            type="button"
            onClick={onClose}
            className="rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onSaveAsNew}
            className="rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
            style={{ background: "linear-gradient(180deg, #3a2f1a, #2a2418)" }}
          >
            Save as new
          </button>
        </footer>
      </div>
    </div>
  );
}

function MiniInputOverlay({
  title,
  sectionLabel,
  label,
  value,
  placeholder,
  submitLabel,
  hint,
  onValueChange,
  onClose,
  onSubmit,
}: {
  title: string;
  sectionLabel: string;
  label: string;
  value: string;
  placeholder?: string;
  submitLabel: string;
  hint?: string;
  onValueChange: (s: string) => void;
  onClose: () => void;
  onSubmit: () => void;
}) {
  return (
    <div
      role="presentation"
      className="fixed inset-0 z-160 flex items-center justify-center backdrop-blur-[2px]"
      onMouseDown={onClose}
      style={{ background: "rgba(0,0,0,0.55)" }}
    >
      <div
        role="dialog"
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
        className="relative flex w-115 max-w-[92vw] flex-col overflow-hidden rounded-md border border-border"
        style={{
          background:
            "linear-gradient(180deg, var(--color-panel-2), color-mix(in srgb, var(--color-bg) 80%, transparent))",
          boxShadow: "0 24px 64px rgba(0,0,0,0.7)",
        }}
      >
        <CornerMarks />
        <header
          className="flex items-center justify-between gap-3 border-b border-border px-5 py-4"
          style={{
            background:
              "linear-gradient(180deg, rgba(201,165,90,0.05), transparent)",
          }}
        >
          <div>
            <div className="mb-1 font-mono text-[10px] uppercase tracking-[0.2em] text-faint">
              {sectionLabel}
            </div>
            <h3 className="m-0 text-[16px] font-semibold uppercase tracking-[0.14em] text-accent-hot">
              {title}
            </h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-[3px] border border-border-2 bg-panel-2 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Close
          </button>
        </header>
        <div className="flex flex-col gap-3 p-5">
          <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
            {label}
          </span>
          <input
            autoFocus
            value={value}
            onChange={(e) => onValueChange(e.target.value)}
            placeholder={placeholder}
            onKeyDown={(e) => {
              if (e.key === "Enter") onSubmit();
              if (e.key === "Escape") onClose();
            }}
            className="w-full rounded-[3px] border border-border-2 px-3 py-2 text-text placeholder:text-faint focus:border-accent-deep focus:outline-none focus:ring-2 focus:ring-accent-hot/15"
            style={{
              background:
                "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
              boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
            }}
          />
          {hint && (
            <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-faint">
              {hint}
            </p>
          )}
        </div>
        <footer className="flex items-center justify-end gap-2 border-t border-border bg-black/30 px-5 py-3">
          <button
            type="button"
            onClick={onClose}
            className="rounded-[3px] border border-border-2 bg-transparent px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onSubmit}
            className="rounded-[3px] border border-accent-deep px-3.5 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-accent-hot transition-colors hover:border-accent-hot hover:text-[#fff0c4]"
            style={{ background: "linear-gradient(180deg, #3a2f1a, #2a2418)" }}
          >
            {submitLabel}
          </button>
        </footer>
      </div>
    </div>
  );
}

const CONTEXT_ITEM_CLASS =
  "flex w-full items-center gap-2 px-3 py-1.5 text-left font-mono text-[11px] tracking-[0.06em] text-text transition-colors hover:bg-accent-hot/10 hover:text-accent-hot";
const CONTEXT_ITEM_DANGER_CLASS =
  "flex w-full items-center gap-2 px-3 py-1.5 text-left font-mono text-[11px] tracking-[0.06em] text-muted transition-colors hover:bg-stat-red/10 hover:text-stat-red";

function ContextMenu({
  state,
  build,
  profile,
  isActiveBuild,
  isActiveProfile,
  armedBuild,
  armedProfile,
  isLastProfile,
  onClose,
  onLoadBuild,
  onUpdateBuild,
  onRenameBuild,
  onDeleteBuild,
  onActivateProfile,
  onRenameProfile,
  onDuplicateProfile,
  onDeleteProfile,
}: {
  state: ContextMenuState;
  build: SavedBuild;
  profile: SavedProfile | null;
  isActiveBuild: boolean;
  isActiveProfile: boolean;
  armedBuild: boolean;
  armedProfile: boolean;
  isLastProfile: boolean;
  onClose: () => void;
  onLoadBuild: () => void;
  onUpdateBuild: () => void;
  onRenameBuild: () => void;
  onDeleteBuild: () => void;
  onActivateProfile: () => void;
  onRenameProfile: () => void;
  onDuplicateProfile: () => void;
  onDeleteProfile: () => void;
}) {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    function onDown() {
      onClose();
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [onClose]);

  return createPortal(
    <div
      role="menu"
      onMouseDown={(e) => e.stopPropagation()}
      className="fixed z-200 flex min-w-55 flex-col overflow-hidden rounded-sm border border-accent-deep/60 py-1"
      style={{
        left: state.x,
        top: state.y,
        background: "var(--color-panel)",
        boxShadow:
          "0 12px 32px rgba(0,0,0,0.65), inset 0 1px 0 rgba(255,255,255,0.03)",
      }}
    >
      <div className="border-b border-border px-3 py-1.5 font-mono text-[9px] uppercase tracking-[0.18em] text-faint">
        {state.kind === "build" ? build.name : (profile?.name ?? "")}
      </div>
      {state.kind === "build" && (
        <>
          {!isActiveBuild && (
            <button
              type="button"
              onClick={onLoadBuild}
              className={CONTEXT_ITEM_CLASS}
            >
              Load build
            </button>
          )}
          {isActiveBuild && (
            <button
              type="button"
              onClick={onUpdateBuild}
              className={CONTEXT_ITEM_CLASS}
            >
              Update active profile
            </button>
          )}
          <button
            type="button"
            onClick={onRenameBuild}
            className={CONTEXT_ITEM_CLASS}
          >
            Rename build…
          </button>
          <button
            type="button"
            onClick={onDeleteBuild}
            className={CONTEXT_ITEM_DANGER_CLASS}
          >
            {armedBuild ? "Confirm delete?" : "Delete build"}
          </button>
        </>
      )}
      {state.kind === "profile" && profile && (
        <>
          {!isActiveProfile && (
            <button
              type="button"
              onClick={onActivateProfile}
              className={CONTEXT_ITEM_CLASS}
            >
              Activate profile
            </button>
          )}
          <button
            type="button"
            onClick={onRenameProfile}
            className={CONTEXT_ITEM_CLASS}
          >
            Rename profile…
          </button>
          <button
            type="button"
            onClick={onDuplicateProfile}
            className={CONTEXT_ITEM_CLASS}
          >
            Duplicate profile
          </button>
          <button
            type="button"
            disabled={isLastProfile}
            onClick={onDeleteProfile}
            className={`${CONTEXT_ITEM_DANGER_CLASS} disabled:cursor-not-allowed disabled:opacity-40`}
          >
            {armedProfile
              ? "Confirm delete?"
              : isLastProfile
                ? "Delete profile (last)"
                : "Delete profile"}
          </button>
        </>
      )}
    </div>,
    document.body,
  );
}

function CornerMarks() {
  const base: React.CSSProperties = {
    position: "absolute",
    width: 12,
    height: 12,
    border: "1px solid var(--color-accent-deep)",
    opacity: 0.6,
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

function PlusIcon() {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      aria-hidden
    >
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

function UploadIcon() {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="17 8 12 3 7 8" />
      <line x1="12" y1="3" x2="12" y2="15" />
    </svg>
  );
}

function SaveIcon() {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
      <polyline points="17 21 17 13 7 13 7 21" />
      <polyline points="7 3 7 8 15 8" />
    </svg>
  );
}

function classColor(classId: string): string {
  let hash = 0;
  for (let i = 0; i < classId.length; i++) {
    hash = (hash * 31 + classId.charCodeAt(i)) | 0;
  }
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue} 58% 58%)`;
}

function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const sameDay =
      d.getFullYear() === now.getFullYear() &&
      d.getMonth() === now.getMonth() &&
      d.getDate() === now.getDate();
    const time = d.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    });
    if (sameDay) return `Today, ${time}`;
    const yesterday = new Date(now);
    yesterday.setDate(now.getDate() - 1);
    if (
      d.getFullYear() === yesterday.getFullYear() &&
      d.getMonth() === yesterday.getMonth() &&
      d.getDate() === yesterday.getDate()
    ) {
      return `Yesterday, ${time}`;
    }
    const day = d.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
    return `${day}, ${time}`;
  } catch {
    return iso;
  }
}
