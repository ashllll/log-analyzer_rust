import { useRef, useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PopoverSurface } from "../PopoverSurface";

function Harness() {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  return (
    <div className="relative">
      <button ref={triggerRef} onClick={() => setOpen(true)}>
        Keyword Groups
      </button>
      <PopoverSurface
        open={open}
        onClose={() => setOpen(false)}
        triggerRef={triggerRef}
        ariaLabel="Keyword Groups"
      >
        <button>First rule</button>
      </PopoverSurface>
    </div>
  );
}

describe("PopoverSurface", () => {
  it("closes on Escape and restores focus to its trigger", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const trigger = screen.getByRole("button", { name: "Keyword Groups" });
    await user.click(trigger);
    await user.click(screen.getByRole("button", { name: "First rule" }));

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("closes when the outside surface is pressed", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Keyword Groups" }));

    await user.click(screen.getByTestId("popover-outside"));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
