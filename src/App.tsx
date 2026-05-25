import { Suspense, lazy, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence, motion } from "motion/react";
import { EASE_OUT, hoverTap, viewVariants } from "./lib/motion";
import BottomBar from "./components/BottomBar";
import BuildsMenu from "./components/BuildsMenu";
import LeftStatsPanel from "./components/LeftStatsPanel";
import { HoverProvider } from "./contexts/HoverProvider";
import Logo from "./components/Logo";
import ShareButton from "./components/ShareButton";
import StartupBuildModal from "./components/StartupBuildModal";
import StorageErrorBanner from "./components/StorageErrorBanner";
import { classes, getClass } from "./data";
import { useBuild } from "./store/build";
import { preloadSprites } from "./utils/preloadAssets";
import { readStorageWithLegacy, writeStorage } from "./utils/storage";
const CharacterView = lazy(() => import("./views/CharacterView"));
const ConfigView = lazy(() => import("./views/ConfigView"));
const GearView = lazy(() => import("./views/gear/GearView"));
const NotesView = lazy(() => import("./views/NotesView"));
const SkillsView = lazy(() => import("./views/SkillsView"));
const StatsView = lazy(() => import("./views/StatsView"));
const TreeView = lazy(() => import("./views/TreeView"));

declare global {
  interface Window {
    __bootProgress?: (pct: number, status?: string) => void;
    __bootFinish?: () => void;
  }
}

const SECTIONS = [
  { id: "character", label: "Character", view: CharacterView },
  { id: "tree", label: "Tree", view: TreeView },
  { id: "skills", label: "Skills", view: SkillsView },
  { id: "gear", label: "Gear", view: GearView },
  { id: "stats", label: "Stats", view: StatsView },
  { id: "config", label: "Config", view: ConfigView },
  { id: "notes", label: "Notes", view: NotesView },
] as const;

type Section = (typeof SECTIONS)[number]["id"];

const SECTION_KEY = "hsplanner.activeSection.v1";
const LEGACY_SECTION_KEY = "heroplanner.activeSection.v1";
const SECTION_IDS = new Set<Section>(SECTIONS.map((s) => s.id));

function readInitialSection(): Section {
  // Legacy "heroplanner" key was renamed at 0.4.x.
  const stored = readStorageWithLegacy(SECTION_KEY, LEGACY_SECTION_KEY);
  if (stored && SECTION_IDS.has(stored as Section)) return stored as Section;
  return "tree";
}

// 0–50% Rust warm-up, 50–100% sprite fetches.
const WARMUP_WEIGHT = 0.5;
const SPRITES_WEIGHT = 0.5;

function ViewLoadingFallback() {
  return (
    <div className="flex h-full items-center justify-center">
      <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
        <span
          aria-hidden
          className="inline-block h-1.5 w-1.5 rotate-45 bg-accent-hot animate-pulse"
          style={{ boxShadow: "0 0 8px rgba(224,184,100,0.6)" }}
        />
        Loading
      </div>
    </div>
  );
}

function App() {
  const [section, setSection] = useState<Section>(readInitialSection);
  const [showStartup, setShowStartup] = useState(false);

  // Boot: warm calc caches and preload sprites while the HTML splash from index.html is visible.
  // Splash listens via window.__bootProgress / window.__bootFinish.
  useEffect(() => {
    // Floor display time to avoid flashing the splash on hot reload.
    const MIN_DISPLAY_MS = 1200;
    let cancelled = false;
    const bootStart = performance.now();

    const report = (pct: number, status?: string) => {
      window.__bootProgress?.(pct, status);
    };

    (async () => {
      report(2, "Loading game data");
      try {
        // Pre-initialise Rust data + parser caches so the first real calc isn't slow.
        await invoke<boolean>("calc_warmup");
      } catch {
        // No Tauri (plain browser) — keep going.
      }
      if (cancelled) return;
      report(WARMUP_WEIGHT * 100, "Loading sprites");

      await preloadSprites((loaded, total) => {
        if (cancelled) return;
        const fraction = total > 0 ? loaded / total : 1;
        report(
          (WARMUP_WEIGHT + SPRITES_WEIGHT * fraction) * 100,
          "Loading sprites",
        );
      });
      if (cancelled) return;
      report(100, "Ready");

      const elapsed = performance.now() - bootStart;
      const remaining = Math.max(0, MIN_DISPLAY_MS - elapsed);
      if (remaining > 0) {
        await new Promise((r) => window.setTimeout(r, remaining));
      }
      if (cancelled) return;
      window.__bootFinish?.();
      setShowStartup(true);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    writeStorage(SECTION_KEY, section);
  }, [section]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        if (section === "tree" || section === "stats") {
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
  const level = useBuild((s) => s.level);
  const setClass = useBuild((s) => s.setClass);
  const setLevel = useBuild((s) => s.setLevel);
  const cls = classId ? getClass(classId) : undefined;

  const needsScroll = section !== "tree" && section !== "skills";

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

        <nav className="flex h-full items-stretch">
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
          {classes.length > 0 && (
            <>
              <label className="flex items-center gap-2">
                <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
                  Class
                </span>
                <div
                  className="inline-flex items-center rounded-[3px] border border-border-2 px-2 py-1 transition-colors hover:border-accent-deep focus-within:border-accent-hot"
                  style={{
                    background:
                      "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
                    boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
                  }}
                >
                  <select
                    value={classId ?? ""}
                    onChange={(e) => setClass(e.target.value)}
                    className="min-w-20 cursor-pointer bg-transparent text-[12px] text-text outline-none"
                  >
                    {classes.map((c) => (
                      <option key={c.id} value={c.id} className="bg-panel">
                        {c.name}
                      </option>
                    ))}
                  </select>
                </div>
              </label>
              <label className="flex items-center gap-2">
                <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
                  Level
                </span>
                <div
                  className="inline-flex items-center rounded-[3px] border border-border-2 px-2 py-1 transition-colors hover:border-accent-deep focus-within:border-accent-hot"
                  style={{
                    background:
                      "linear-gradient(180deg, #0d0e12, var(--color-panel-2))",
                    boxShadow: "inset 0 1px 2px rgba(0,0,0,0.5)",
                  }}
                >
                  <input
                    type="number"
                    min={1}
                    value={level}
                    onChange={(e) => setLevel(Number(e.target.value))}
                    className="w-14 bg-transparent text-center font-mono text-[12px] text-accent-hot tabular-nums outline-none"
                  />
                </div>
              </label>
            </>
          )}
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
          <span aria-hidden className="h-6 w-px bg-border" />
          <BuildsMenu />
          <ShareButton />
        </div>
      </header>

      <div className="flex min-h-0 flex-1 overflow-hidden">
        <LeftStatsPanel />
        <main
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
              <Suspense fallback={<ViewLoadingFallback />}>
                <ActiveView />
              </Suspense>
            </motion.div>
          </AnimatePresence>
        </main>
      </div>

      <BottomBar />

      <StartupBuildModal
        isOpen={showStartup}
        onClose={() => setShowStartup(false)}
      />

      <StorageErrorBanner />
    </div>
    </HoverProvider>
  );
}

export default App;
