import { useCallback, useEffect, useRef } from "react";
import { cn } from "../../utils/classNames";

interface PopoverSurfaceProps {
  open: boolean;
  onClose: () => void;
  triggerRef: React.RefObject<HTMLElement | null>;
  ariaLabel: string;
  id?: string;
  className?: string;
  children: React.ReactNode;
}

export function PopoverSurface({
  open,
  onClose,
  triggerRef,
  ariaLabel,
  id,
  className,
  children,
}: PopoverSurfaceProps) {
  const surfaceRef = useRef<HTMLDivElement>(null);
  const closingRef = useRef(false);

  const close = useCallback(() => {
    if (closingRef.current) return;
    closingRef.current = true;
    const surface = surfaceRef.current;
    const finish = () => {
      onClose();
      triggerRef.current?.focus();
    };
    if (
      !surface ||
      !surface.animate ||
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      finish();
      return;
    }
    surface
      .animate(
        [
          { opacity: 1, transform: "translateY(0) scale(1)" },
          { opacity: 0, transform: "translateY(-2px) scale(.98)" },
        ],
        {
          duration: 130,
          easing: "cubic-bezier(.23, 1, .32, 1)",
          fill: "forwards",
        }
      )
      .finished.then(finish, finish);
  }, [onClose, triggerRef]);

  useEffect(() => {
    if (!open) return;
    closingRef.current = false;
    const focusTimer = window.setTimeout(() => {
      surfaceRef.current
        ?.querySelector<HTMLElement>(
          'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])'
        )
        ?.focus();
    }, 0);
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      window.clearTimeout(focusTimer);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [close, open]);

  if (!open) return null;

  return (
    <>
      <button
        type="button"
        aria-label="Close popover"
        data-testid="popover-outside"
        tabIndex={-1}
        className="fixed inset-0 z-[45] cursor-default bg-transparent"
        onClick={close}
      />
      <div
        ref={surfaceRef}
        id={id}
        role="dialog"
        aria-label={ariaLabel}
        className={cn(
          "popover-surface apple-material motion-spatial absolute right-0 top-full z-50 mt-2",
          className
        )}
      >
        {children}
      </div>
    </>
  );
}
