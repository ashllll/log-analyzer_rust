import { createRef, useState } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { DialogSurface } from "../DialogSurface";

describe("DialogSurface", () => {
  it("traps focus, closes on Escape, and restores the previous focus", async () => {
    function Harness() {
      const [open, setOpen] = useState(false);
      const initialFocusRef = createRef<HTMLInputElement>();
      return (
        <>
          <button type="button" onClick={() => setOpen(true)}>
            Open settings
          </button>
          <DialogSurface
            open={open}
            onClose={() => setOpen(false)}
            ariaLabel="Settings"
            initialFocusRef={initialFocusRef}
          >
            <input ref={initialFocusRef} aria-label="First field" />
            <button type="button">Done</button>
          </DialogSurface>
        </>
      );
    }
    render(<Harness />);

    const trigger = screen.getByRole("button", { name: "Open settings" });
    trigger.focus();
    fireEvent.click(trigger);
    await waitFor(() =>
      expect(screen.getByLabelText("First field")).toHaveFocus()
    );
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(screen.getByRole("button", { name: "Done" })).toHaveFocus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(screen.getByLabelText("First field")).toHaveFocus();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(trigger).toHaveFocus();
  });

  it("closes when the scrim is pressed", () => {
    const onClose = jest.fn();
    render(
      <DialogSurface open onClose={onClose} ariaLabel="Settings">
        <button>Done</button>
      </DialogSurface>
    );
    fireEvent.mouseDown(screen.getByTestId("dialog-scrim"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("routes marked close controls through the shared close behavior", () => {
    const onClose = jest.fn();
    render(
      <DialogSurface open onClose={onClose} ariaLabel="Settings">
        <button data-dialog-close>Cancel</button>
      </DialogSurface>
    );
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("restores focus when its parent closes it directly", () => {
    const trigger = document.createElement("button");
    document.body.appendChild(trigger);
    trigger.focus();
    const { rerender } = render(
      <DialogSurface open onClose={jest.fn()} ariaLabel="Settings">
        <button>Done</button>
      </DialogSurface>
    );
    rerender(
      <DialogSurface open={false} onClose={jest.fn()} ariaLabel="Settings">
        <button>Done</button>
      </DialogSurface>
    );
    expect(trigger).toHaveFocus();
    trigger.remove();
  });

  it("removes spatial exit motion when reduced motion is requested", () => {
    const onClose = jest.fn();
    const animate = jest.fn();
    HTMLElement.prototype.animate = animate;
    window.matchMedia = jest.fn().mockReturnValue({ matches: true });
    render(
      <DialogSurface open onClose={onClose} ariaLabel="Settings">
        <button>Done</button>
      </DialogSurface>
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(animate).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
