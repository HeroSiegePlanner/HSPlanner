import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AnimatePresence, motion } from "motion/react";
import { EASE_OUT, hoverTap, viewVariants } from "./utils/motion";
import BottomBar from "./components/app/BottomBar";
import BuildsMenu from "./components/app/BuildsMenu";
import { AUTO_OPEN_KEY, BuildSelect } from "./components/buildSelect";
import { DeepLinkPrompt, type DeepLinkPromptState } from "./components/app/DeepLinkPrompt";
import LeftStatsPanel from "./components/app/LeftStatsPanel";
import { HoverProvider } from "./contexts/HoverProvider";
import Logo from "./components/ui/Logo";
import SeasonErrorBanner from "./components/app/SeasonErrorBanner";
import SeasonSwitcher from "./components/app/SeasonSwitcher";
import SeasonToast from "./components/app/SeasonToast";
import SettingsModal from "./components/app/SettingsModal";
import ShareButton from "./components/app/ShareButton";
import StorageErrorBanner from "./components/app/StorageErrorBanner";
import TutorialOverlay from "./components/app/TutorialOverlay";
import { TUTORIAL_DONE_KEY } from "./components/app/tutorialModel";
import { activeSeasonId, getClass } from "@data";
import { PENDING_BUILD_KEY, PENDING_IMPORT_KEY } from "@data/seasons/registry";
import { initAutoSave } from "./store/autoSave";
import { initUiZoom } from "./store/settings";
import { useBuild } from "./store/build";
import { initUndoHistory, redoLastChange, undoLastChange } from "./store/undoHistory";
import { initShiftScroll } from "./utils/shiftScroll";
import { createDeepLinkDispatcher, getInitialDeepLinkUrls } from "./utils/build/deepLink";
import { listSavedBuilds } from "./utils/build/savedBuilds";
import { decodeShareToBuild, type DecodedShare } from "./utils/build/shareBuild";
import { spriteBootProgress, warmupBootProgress } from "./utils/bootProgress";
import { preloadSprites } from "./utils/preloadAssets";
import { readStorage, readStorageWithLegacy, removeStorage, writeStorage } from "./utils/storage";
// ponytail: static imports — every view ships in the main bundle, which the boot
// splash already covers, so switching tabs never waits on a chunk
import CharacterView from "./views/CharacterView";
import ConfigView from "./views/ConfigView";
import EtherView from "./views/EtherView";
import FiltersView from "./views/filters/FiltersView";
import GearView from "./views/gear/GearView";
import MercView from "./views/MercView";
import NotesView from "./views/NotesView";
import SkillsView from "./views/SkillsView";
import StatsView from "./views/StatsView";
import TreeView from "./views/TreeView";

declare global {
  interface Window {
    __bootProgress?: (pct: number, status?: string) => void;
    __bootFinish?: () => void;
  }
}

const SECTIONS = [
  { id: "character", label: "Character", view: CharacterView },
  { id: "tree", label: "Tree", view: TreeView },
  { id: "ether", label: "Ether", view: EtherView },
  { id: "skills", label: "Skills", view: SkillsView },
  { id: "gear", label: "Gear", view: GearView },
  { id: "merc", label: "Merc", view: MercView },
  { id: "stats", label: "Stats", view: StatsView },
  { id: "config", label: "Config", view: ConfigView },
  { id: "notes", label: "Notes", view: NotesView },
  { id: "filters", label: "Filters", view: FiltersView },
] as const;

export type Section = (typeof SECTIONS)[number]["id"];

type Screen = "library" | "planner";

const SECTION_KEY = "hsplanner.activeSection.v1";
const LEGACY_SECTION_KEY = "heroplanner.activeSection.v1";
const SECTION_IDS = new Set<Section>(SECTIONS.map((s) => s.id));

function readInitialSection(): Section {
  const stored = readStorageWithLegacy(SECTION_KEY, LEGACY_SECTION_KEY);
  if (stored && SECTION_IDS.has(stored as Section)) return stored as Section;
  return "tree";
}

function App() {
  const [section, setSection] = useState<Section>(readInitialSection);
  const [screen, setScreen] = useState<Screen>("library");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [tutorialOpen, setTutorialOpen] = useState(false);
  const [deepLinkPrompt, setDeepLinkPrompt] = useState<DeepLinkPromptState | null>(null);

  const enterPlanner = useCallback(() => {
    setScreen("planner");
    if (!readStorage(TUTORIAL_DONE_KEY)) setTutorialOpen(true);
  }, []);

  useEffect(() => {
    const onReady = (code: string, decoded: DecodedShare) => {
      const cls = decoded.snapshot.classId ? getClass(decoded.snapshot.classId) : undefined;
      setDeepLinkPrompt({
        kind: "confirm",
        title: cls?.name ?? "Shared build",
        onConfirm: () => {
          const record = useBuild.getState().importCodeToLibrary(code);
          if (record) setScreen("library");
        },
      });
    };
    const onError = (message: string) => setDeepLinkPrompt({ kind: "error", message });

    const { dispatchInitial, dispatchLive } = createDeepLinkDispatcher(onReady, onError);
    void getInitialDeepLinkUrls().then(dispatchInitial);
    const unlistenPromise = listen<string[]>("deep-link://new-url", (event) => {
      void dispatchLive(event.payload);
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const MIN_DISPLAY_MS = 500;
    const FINALIZE_RESERVE = 1;
    let cancelled = false;
    const bootStart = performance.now();

    let lastPct = 0;
    const report = (pct: number, status?: string) => {
      lastPct = Math.max(lastPct, pct);
      window.__bootProgress?.(lastPct, status);
    };

    (async () => {
      report(0, "Loading game data");
      const warmupTask = (async () => {
        try {
          const unlisten = await listen<{ current: number; total: number }>(
            "warmup-progress",
            (e) => {
              if (cancelled) return;
              const { pct } = warmupBootProgress(
                e.payload.current,
                e.payload.total,
              );
              report(pct);
            },
          );
          try {
            await invoke<boolean>("calc_warmup", { season: activeSeasonId });
          } finally {
            unlisten();
          }
        } catch (err) {
          void err;
        }
      })();
      const spritesTask = preloadSprites((loaded, total) => {
        if (cancelled) return;
        const { pct, status } = spriteBootProgress(loaded, total);
        report(Math.min(pct, 100 - FINALIZE_RESERVE), status);
      });
      await Promise.all([warmupTask, spritesTask]);
      if (cancelled) return;

      const remaining = Math.max(0, MIN_DISPLAY_MS - (performance.now() - bootStart));
      if (remaining > 0) {
        await new Promise((r) => window.setTimeout(r, remaining));
      }
      if (cancelled) return;
      report(100, "Ready");
      window.__bootFinish?.();

      const pendingBuild = readStorage(PENDING_BUILD_KEY);
      const pendingImport = readStorage(PENDING_IMPORT_KEY);
      if (pendingBuild) {
        removeStorage(PENDING_BUILD_KEY);
        if (useBuild.getState().loadSavedBuild(pendingBuild)) enterPlanner();
      } else if (pendingImport) {
        removeStorage(PENDING_IMPORT_KEY);
        const decoded = decodeShareToBuild(pendingImport);
        if (decoded) {
          useBuild.getState().importBuildSnapshot(decoded.snapshot, decoded.notes);
          enterPlanner();
        }
      } else if (readStorage(AUTO_OPEN_KEY) === "1") {
        const recent = listSavedBuilds()[0];
        if (recent && useBuild.getState().loadSavedBuild(recent.id)) {
          enterPlanner();
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [enterPlanner]);

  useEffect(() => {
    writeStorage(SECTION_KEY, section);
  }, [section]);

  useEffect(() => initAutoSave(), []);
  useEffect(() => initUndoHistory(), []);
  useEffect(() => initShiftScroll(), []);
  useEffect(() => initUiZoom(), []);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      if ((e.ctrlKey || e.metaKey) && (key === "z" || key === "y")) {
        const t = e.target;
        const isEditable =
          t instanceof HTMLElement &&
          (t.isContentEditable || t.tagName === "INPUT" || t.tagName === "TEXTAREA");
        if (isEditable) return; // leave native text undo/redo alone
        e.preventDefault();
        if (key === "y" || e.shiftKey) redoLastChange();
        else undoLastChange();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        useBuild.getState().saveBuildNow();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        if (
          section === "tree" ||
          section === "ether" ||
          section === "stats"
        ) {
          const input = document.querySelector<HTMLInputElement>(
            "[data-search-input]",
          );
          if (input) {
            input.focus();
            input.select();
          }
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [section]);

  const ActiveView = SECTIONS.find((s) => s.id === section)?.view ?? TreeView;
  const classId = useBuild((s) => s.classId);
  const activeBuildId = useBuild((s) => s.activeBuildId);
  const cls = classId ? getClass(classId) : undefined;

  const needsScroll =
    section !== "tree" && section !== "skills" && section !== "ether";

  const openBuild = (buildId: string) => {
    if (useBuild.getState().loadSavedBuild(buildId)) {
      enterPlanner();
    }
  };
  const overlays = (
    <>
      <StorageErrorBanner />
      {deepLinkPrompt && (
        <DeepLinkPrompt state={deepLinkPrompt} onClose={() => setDeepLinkPrompt(null)} />
      )}
    </>
  );

  if (screen === "library") {
    return (
      <HoverProvider>
        <BuildSelect
          onOpenBuild={openBuild}
          onClose={() => enterPlanner()}
          canClose={activeBuildId != null}
        />
        {overlays}
      </HoverProvider>
    );
  }

  return (
    <HoverProvider>
      <div className="flex h-screen w-screen flex-col bg-bg text-text">
        <header
          className="relative flex h-11 shrink-0 items-center gap-0 border-b border-border pl-3 pr-3"
          style={{
            background:
              "linear-gradient(180deg, var(--color-panel-2), var(--color-panel))",
            boxShadow:
              "inset 0 -1px 0 rgba(201,165,90,0.08), 0 1px 0 rgba(0,0,0,0.4)",
          }}
        >
          <div className="mr-3 flex items-center gap-2 border-r border-border pr-3">
            <Logo size={22} glow title="HSPlanner" />
            <span
              className="select-none font-mono text-[11px] uppercase tracking-[0.18em] text-accent-hot"
              style={{ textShadow: "0 0 10px rgba(224,184,100,0.25)" }}
            >
              HSPlanner
            </span>
          </div>

          <nav data-tour="sections" className="flex h-full items-stretch">
            {SECTIONS.map((s) => {
              const active = section === s.id;
              return (
                <motion.button
                  key={s.id}
                  onClick={() => setSection(s.id)}
                  {...hoverTap}
                  className={`group relative flex h-full items-center gap-2 px-3.5 font-mono text-[11px] uppercase tracking-[0.16em] transition-colors ${
                    active
                      ? "text-accent-hot"
                      : "text-muted hover:text-text"
                  }`}
                >
                  <span
                    aria-hidden
                    className={`inline-block h-1.5 w-1.5 rotate-45 transition-all ${
                      active
                        ? "bg-accent-hot"
                        : "bg-faint group-hover:bg-muted"
                    }`}
                    style={
                      active
                        ? { boxShadow: "0 0 8px rgba(224,184,100,0.6)" }
                        : undefined
                    }
                  />
                  {s.label}
                  {active && (
                    <motion.span
                      layoutId="tab-underline"
                      aria-hidden
                      className="pointer-events-none absolute bottom-0 left-2 right-2 h-[2px]"
                      style={{
                        background:
                          "linear-gradient(90deg, transparent, var(--color-accent-hot), transparent)",
                        boxShadow: "0 0 12px rgba(224,184,100,0.45)",
                      }}
                      transition={{ duration: 0.16, ease: EASE_OUT }}
                    />
                  )}
                </motion.button>
              );
            })}
          </nav>

          <div className="ml-auto flex items-center gap-2.5">
            {cls?.primaryAttribute && (
              <span
                className="hidden items-center gap-1.5 rounded-[3px] border border-accent-deep/40 px-2 py-1 font-mono text-[10px] uppercase tracking-[0.18em] text-accent-hot md:inline-flex"
                style={{
                  background:
                    "linear-gradient(180deg, rgba(58,46,24,0.6), rgba(42,36,24,0.4))",
                }}
              >
                <span
                  aria-hidden
                  className="inline-block h-1 w-1 rotate-45 bg-accent-hot"
                  style={{ boxShadow: "0 0 6px rgba(224,184,100,0.6)" }}
                />
                {cls.primaryAttribute}
              </span>
            )}
            <SeasonSwitcher />
            <span aria-hidden className="h-6 w-px bg-border" />
            <BuildsMenu onOpenLibrary={() => setScreen("library")} />
            <ShareButton />
            <button
              type="button"
              onClick={() => setTutorialOpen(true)}
              title="Tutorial"
              aria-label="Open tutorial"
              className="rounded-[3px] border border-border-2 px-2.5 py-1 font-mono text-[12px] leading-[1.35] text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              ?
            </button>
            <button
              type="button"
              onClick={() => setSettingsOpen(true)}
              title="Settings"
              aria-label="Settings"
              data-tour="settings"
              className="rounded-[3px] border border-border-2 p-1.5 text-muted transition-colors hover:border-accent-deep hover:text-accent-hot"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden
                className="h-3.5 w-3.5"
              >
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </button>
          </div>
        </header>

        <SeasonErrorBanner />
        <SeasonToast />

        <div className="flex min-h-0 flex-1 overflow-hidden">
          <LeftStatsPanel />
          <main
            data-tour="view"
            className={`flex-1 min-w-0 ${needsScroll ? "overflow-auto p-6" : "overflow-hidden"}`}
          >
            <AnimatePresence mode="wait" initial={false}>
              <motion.div
                key={section}
                variants={viewVariants}
                initial="initial"
                animate="animate"
                exit="exit"
                className="h-full"
              >
                <ActiveView />
              </motion.div>
            </AnimatePresence>
          </main>
        </div>

        <BottomBar />
      </div>

      {settingsOpen && <SettingsModal onClose={() => setSettingsOpen(false)} />}
      {tutorialOpen && (
        <TutorialOverlay
          section={section}
          setSection={setSection}
          onClose={() => setTutorialOpen(false)}
        />
      )}
      {overlays}
    </HoverProvider>
  );
}

export default App;
