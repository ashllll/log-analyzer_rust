import {
  useRef,
  useState,
  type HTMLAttributes,
  type PointerEvent,
} from "react";

const DEFAULT_WIDTH = 420;
const MIN_WIDTH = 320;
const MAX_WIDTH = 640;

export function useResizableInspector() {
  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const drag = useRef<{
    pointerId: number;
    startX: number;
    startWidth: number;
  } | null>(null);

  const end = (event: PointerEvent<HTMLDivElement>) => {
    if (drag.current?.pointerId !== event.pointerId) return;
    event.currentTarget.releasePointerCapture?.(event.pointerId);
    drag.current = null;
  };

  const handleProps: HTMLAttributes<HTMLDivElement> = {
    role: "separator",
    tabIndex: 0,
    "aria-label": "Resize log details",
    "aria-orientation": "vertical",
    "aria-valuemin": MIN_WIDTH,
    "aria-valuemax": MAX_WIDTH,
    "aria-valuenow": width,
    onKeyDown: (event) => {
      const step = event.shiftKey ? 32 : 16;
      if (event.key === "ArrowLeft")
        setWidth((current) => Math.min(MAX_WIDTH, current + step));
      else if (event.key === "ArrowRight")
        setWidth((current) => Math.max(MIN_WIDTH, current - step));
      else if (event.key === "Home") setWidth(MIN_WIDTH);
      else if (event.key === "End") setWidth(MAX_WIDTH);
      else return;
      event.preventDefault();
    },
    onPointerDown: (event) => {
      if (drag.current) return;
      drag.current = {
        pointerId: event.pointerId,
        startX: event.clientX,
        startWidth: width,
      };
      event.currentTarget.setPointerCapture?.(event.pointerId);
    },
    onPointerMove: (event) => {
      if (drag.current?.pointerId !== event.pointerId) return;
      const nextWidth =
        drag.current.startWidth + drag.current.startX - event.clientX;
      setWidth(Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, nextWidth)));
    },
    onPointerUp: end,
    onPointerCancel: end,
  };

  return { width, handleProps };
}
