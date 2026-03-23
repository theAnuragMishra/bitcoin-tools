import { useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

export default function Tooltip({
  children,
  tip,
}: {
  children: React.ReactNode;
  tip: string;
}) {
  const triggerRef = useRef<HTMLSpanElement | null>(null);
  const tipRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  useLayoutEffect(() => {
    if (!open || !triggerRef.current || !tipRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    const tipRect = tipRef.current.getBoundingClientRect();
    const gap = 10;
    const viewportW = window.innerWidth;
    const viewportH = window.innerHeight;
    let top = rect.top - tipRect.height - gap;
    let nextPlacement: "top" | "bottom" = "top";
    if (top < 8) {
      top = rect.bottom + gap;
      nextPlacement = "bottom";
    }
    const center = rect.left + rect.width / 2;
    let left = center - tipRect.width / 2;
    left = Math.max(8, Math.min(left, viewportW - tipRect.width - 8));
    if (top + tipRect.height > viewportH - 8) {
      top = Math.max(8, rect.top - tipRect.height - gap);
      nextPlacement = top < rect.top ? "top" : "bottom";
    }
    tipRef.current.style.top = `${top}px`;
    tipRef.current.style.left = `${left}px`;
    tipRef.current.classList.toggle("tt-bottom", nextPlacement === "bottom");
  }, [open, tip]);

  return (
    <span
      className="tt-wrap"
      ref={triggerRef}
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocus={() => setOpen(true)}
      onBlur={() => setOpen(false)}
    >
      {children}
      {open &&
        createPortal(
          <div className="tt" ref={tipRef} role="tooltip">
            {tip}
          </div>,
          document.body,
        )}
    </span>
  );
}
