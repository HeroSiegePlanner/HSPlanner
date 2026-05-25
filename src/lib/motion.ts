// Shared motion presets. Budget: <=0.2s, <=6px travel, <=0.02 scale delta, no springs. Import useReducedMotion from here too.

import type { Transition, Variants } from "motion/react";
import { useReducedMotion } from "motion/react";

export { useReducedMotion };

export const EASE_OUT = [0.22, 0.61, 0.36, 1] as const;
export const EASE_IN_OUT = [0.4, 0, 0.2, 1] as const;

// `motion` uses seconds, not milliseconds.
export const DURATION = {
  fast: 0.12,
  base: 0.16,
  view: 0.2,
} as const;

export const T_FAST: Transition = { duration: DURATION.fast, ease: EASE_OUT };
export const T_BASE: Transition = { duration: DURATION.base, ease: EASE_OUT };
export const T_VIEW: Transition = { duration: DURATION.view, ease: EASE_OUT };

export const viewVariants: Variants = {
  initial: { opacity: 0, y: 4 },
  animate: { opacity: 1, y: 0, transition: T_VIEW },
  exit: { opacity: 0, y: -4, transition: { duration: DURATION.fast, ease: EASE_OUT } },
};

export const backdropVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1, transition: T_BASE },
  exit: { opacity: 0, transition: T_FAST },
};

export const panelVariants: Variants = {
  initial: { opacity: 0, scale: 0.98, y: 6 },
  animate: { opacity: 1, scale: 1, y: 0, transition: T_BASE },
  exit: { opacity: 0, scale: 0.98, y: 6, transition: T_FAST },
};

export const listContainerVariants: Variants = {
  initial: {},
  animate: {
    transition: { staggerChildren: 0.018, delayChildren: 0.02 },
  },
};

// Opacity only: skill icons are absolutely positioned and `y` would fight `top`.
export const skillIconVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1, transition: T_FAST },
};

export const hoverTap = {
  whileHover: { scale: 1.03 },
  whileTap: { scale: 0.97 },
  transition: T_FAST,
} as const;
