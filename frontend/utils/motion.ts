import type { Transition, Variants } from "motion/react";
import { useReducedMotion } from "motion/react";

export { useReducedMotion };

export const EASE_OUT = [0.22, 0.61, 0.36, 1] as const;

export const T_FAST: Transition = { duration: 0.12, ease: EASE_OUT };
export const T_BASE: Transition = { duration: 0.16, ease: EASE_OUT };
export const T_VIEW: Transition = { duration: 0.2, ease: EASE_OUT };

export const viewVariants: Variants = {
  initial: { opacity: 0, y: 4 },
  animate: { opacity: 1, y: 0, transition: T_VIEW },
  exit: { opacity: 0, y: -4, transition: T_FAST },
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

export const skillIconVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1, transition: T_FAST },
};

export const listItemVariants: Variants = {
  initial: { opacity: 0, y: 3 },
  animate: { opacity: 1, y: 0, transition: T_FAST },
};

export const hoverTap = {
  whileHover: { scale: 1.03 },
  whileTap: { scale: 0.97 },
  transition: T_FAST,
} as const;
