// Shared animation presets for HSPlanner.
//
// One source of truth so every animated surface feels the same: subtle and
// fast. All motion stays under ~0.2s, travel under ~6px, scale delta under
// 0.02, and there are no springs — polish for a tool, not a showcase.
//
// Everything motion-related should import from this module (including
// `useReducedMotion`) so there is a single place to tune the feel.

import type { Transition, Variants } from "motion/react";
import { useReducedMotion } from "motion/react";

export { useReducedMotion };

// Cubic-bezier easings. EASE_OUT is a calm decelerate with no overshoot.
export const EASE_OUT = [0.22, 0.61, 0.36, 1] as const;

// Durations in seconds (motion uses seconds, not milliseconds).
const DURATION = {
  fast: 0.12,
  base: 0.16,
  view: 0.2,
} as const;

export const T_FAST: Transition = { duration: DURATION.fast, ease: EASE_OUT };
export const T_BASE: Transition = { duration: DURATION.base, ease: EASE_OUT };
export const T_VIEW: Transition = { duration: DURATION.view, ease: EASE_OUT };

// Crossfade between the main tab views: fade plus a tiny vertical drift.
export const viewVariants: Variants = {
  initial: { opacity: 0, y: 4 },
  animate: { opacity: 1, y: 0, transition: T_VIEW },
  exit: { opacity: 0, y: -4, transition: { duration: DURATION.fast, ease: EASE_OUT } },
};

// Modal backdrop: pure opacity fade, no transform.
export const backdropVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1, transition: T_BASE },
  exit: { opacity: 0, transition: T_FAST },
};

// Modal panel: fade in with a faint scale-up and lift.
export const panelVariants: Variants = {
  initial: { opacity: 0, scale: 0.98, y: 6 },
  animate: { opacity: 1, scale: 1, y: 0, transition: T_BASE },
  exit: { opacity: 0, scale: 0.98, y: 6, transition: T_FAST },
};

// Stagger container for lists / grids. Pair with a child item variant
// (e.g. `skillIconVariants`).
export const listContainerVariants: Variants = {
  initial: {},
  animate: {
    transition: { staggerChildren: 0.018, delayChildren: 0.02 },
  },
};

// Skill icons are absolutely positioned, so a `y` offset would fight their
// `left`/`top`. Opacity only.
export const skillIconVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1, transition: T_FAST },
};

// Generic staggered list item (pair with listContainerVariants). Opacity plus a
// 3px lift — small enough not to jitter a dense list such as the build table.
export const listItemVariants: Variants = {
  initial: { opacity: 0, y: 3 },
  animate: { opacity: 1, y: 0, transition: T_FAST },
};

// Micro-interaction preset for headline buttons. Spread onto a motion element:
//   <motion.button {...hoverTap} />
export const hoverTap = {
  whileHover: { scale: 1.03 },
  whileTap: { scale: 0.97 },
  transition: T_FAST,
} as const;
