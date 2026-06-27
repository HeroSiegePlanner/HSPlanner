import { useEffect, useRef, type ReactNode } from "react";
import { motion, useAnimationControls } from "motion/react";
import { EASE_OUT, useReducedMotion } from "../lib/motion";

interface FlashOnChangeProps {
  value: number;
  className?: string;
  children: ReactNode;
}

export default function FlashOnChange({
  value,
  className,
  children,
}: FlashOnChangeProps) {
  const controls = useAnimationControls();
  const reduced = useReducedMotion();
  const isFirstRender = useRef(true);

  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false;
      return;
    }
    if (reduced) return;
    controls.start({
      opacity: [1, 0.55, 1],
      transition: { duration: 0.18, ease: EASE_OUT },
    });
  }, [value, reduced, controls]);

  return (
    <motion.span animate={controls} className={className}>
      {children}
    </motion.span>
  );
}
