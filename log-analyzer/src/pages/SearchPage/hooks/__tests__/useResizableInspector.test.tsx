import { fireEvent, render, screen } from "@testing-library/react";
import { useResizableInspector } from "../useResizableInspector";

function Harness() {
  const { width, handleProps } = useResizableInspector();
  return (
    <div>
      <output>{width}</output>
      <div data-testid="handle" {...handleProps} />
    </div>
  );
}

describe("useResizableInspector", () => {
  beforeEach(() => {
    HTMLElement.prototype.setPointerCapture = jest.fn();
    HTMLElement.prototype.releasePointerCapture = jest.fn();
  });

  it("tracks the pointer directly and clamps the inspector width", () => {
    render(<Harness />);
    const handle = screen.getByTestId("handle");

    fireEvent(
      handle,
      new MouseEvent("pointerdown", { bubbles: true, clientX: 500 })
    );
    fireEvent(
      handle,
      new MouseEvent("pointermove", { bubbles: true, clientX: 900 })
    );
    expect(screen.getByText("320")).toBeInTheDocument();

    fireEvent(
      handle,
      new MouseEvent("pointermove", { bubbles: true, clientX: -100 })
    );
    expect(screen.getByText("640")).toBeInTheDocument();
    fireEvent(
      handle,
      new MouseEvent("pointerup", { bubbles: true, clientX: -100 })
    );
  });

  it("supports keyboard resizing and exposes the current width", () => {
    render(<Harness />);
    const handle = screen.getByTestId("handle");
    expect(handle).toHaveAttribute("aria-valuenow", "420");
    fireEvent.keyDown(handle, { key: "ArrowLeft" });
    expect(screen.getByText("436")).toBeInTheDocument();
    expect(handle).toHaveAttribute("aria-valuenow", "436");
    fireEvent.keyDown(handle, { key: "Home" });
    expect(screen.getByText("320")).toBeInTheDocument();
  });
});
