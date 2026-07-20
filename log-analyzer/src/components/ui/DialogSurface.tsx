import { useCallback, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { cn } from "../../utils/classNames";

interface DialogSurfaceProps {
  open: boolean;
  onClose: () => void;
  ariaLabel?: string;
  ariaLabelledBy?: string;
  initialFocusRef?: React.RefObject<HTMLElement | null>;
  requestCloseRef?: React.MutableRefObject<(() => void) | null>;
  className?: string;
  children: React.ReactNode;
}

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function DialogSurface({
  open,
  onClose,
  ariaLabel,
  ariaLabelledBy,
  initialFocusRef,
  requestCloseRef,
  className,
  children,
}: DialogSurfaceProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);
  const closingRef = useRef(false);

  const restoreFocus = useCallback(() => previousFocusRef.current?.focus(), []);

  useEffect(() => {
    if (!open) return;
    previousFocusRef.current = document.activeElement as HTMLElement;
    return restoreFocus;
  }, [open, restoreFocus]);

  const requestClose = useCallback(() => {
    if (closingRef.current) return;
    closingRef.current = true;
    const panel = panelRef.current;
    const reduceMotion = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)"
    ).matches;
    if (!panel?.animate || reduceMotion) {
      restoreFocus();
      onClose();
      return;
    }
    panel
      .animate(
        [
          { opacity: 1, transform: "translateY(0) scale(1)" },
          { opacity: 0, transform: "translateY(6px) scale(.985)" },
        ],
        {
          duration: 180,
          easing: "cubic-bezier(.4, 0, .2, 1)",
          fill: "forwards",
        }
      )
      .finished.then(() => {
        restoreFocus();
        onClose();
      })
      .catch(() => {
        restoreFocus();
        onClose();
      });
  }, [onClose, restoreFocus]);

  useEffect(() => {
    if (!requestCloseRef) return;
    requestCloseRef.current = requestClose;
    return () => {
      requestCloseRef.current = null;
    };
  }, [requestClose, requestCloseRef]);

  useEffect(() => {
    if (!open) return;
    closingRef.current = false;
    const focusTimer = window.setTimeout(() => {
      (
        initialFocusRef?.current ??
        panelRef.current?.querySelector<HTMLElement>(FOCUSABLE)
      )?.focus();
    }, 0);
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        requestClose();
        return;
      }
      if (event.key !== "Tab" || !panelRef.current) return;
      const focusable = Array.from(
        panelRef.current.querySelectorAll<HTMLElement>(FOCUSABLE)
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      window.clearTimeout(focusTimer);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [initialFocusRef, open, requestClose]);

  if (!open) return null;
  return createPortal(
    <div
      className="dialog-host fixed inset-0 z-[100] flex items-center justify-center p-6"
      role="presentation"
    >
      <button
        type="button"
        data-testid="dialog-scrim"
        tabIndex={-1}
        aria-label="Close dialog"
        className="dialog-scrim absolute inset-0 cursor-default bg-black/45 backdrop-blur-sm"
        onMouseDown={requestClose}
      />
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        onClickCapture={(event) => {
          if ((event.target as Element).closest("[data-dialog-close]")) {
            event.preventDefault();
            event.stopPropagation();
            requestClose();
          }
        }}
        className={cn(
          "dialog-panel relative flex max-h-[85vh] flex-col overflow-hidden rounded-[16px] border border-border-base bg-bg-card shadow-elevated",
          className
        )}
      >
        {children}
      </div>
    </div>,
    document.body
  );
}
