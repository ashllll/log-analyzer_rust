import { fireEvent, render, screen } from "@testing-library/react";
import { Circle } from "lucide-react";
import { Button, Card, EmptyState, Input } from "..";

describe("UI foundations", () => {
  it("keeps button and input behavior native and accessible", () => {
    const onClick = jest.fn();
    render(
      <>
        <Button onClick={onClick}>Run search</Button>
        <Input aria-label="Query" />
      </>
    );
    fireEvent.click(screen.getByRole("button", { name: "Run search" }));
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(screen.getByLabelText("Query")).toHaveProperty("type", "text");
  });

  it("exposes disabled and loading button states through native semantics", () => {
    const onClick = jest.fn();
    render(
      <>
        <Button disabled onClick={onClick}>
          Disabled
        </Button>
        <Button loading onClick={onClick}>
          Saving
        </Button>
      </>
    );

    expect(screen.getByRole("button", { name: "Disabled" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Saving" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Disabled" }));
    expect(onClick).not.toHaveBeenCalled();
  });

  it.each([
    ["primary", "bg-primary"],
    ["secondary", "bg-bg-card"],
    ["danger", "text-status-error"],
    ["ghost", "hover:bg-bg-hover"],
  ] as const)(
    "maps the %s variant to semantic theme tokens",
    (variant, tokenClass) => {
      render(<Button variant={variant}>{variant}</Button>);
      expect(screen.getByRole("button", { name: variant })).toHaveClass(
        tokenClass
      );
    }
  );

  it("preserves native input error and disabled accessibility state", () => {
    render(<Input aria-label="Path" aria-invalid="true" disabled />);
    const input = screen.getByLabelText("Path");
    expect(input).toBeDisabled();
    expect(input).toHaveAttribute("aria-invalid", "true");
  });

  it("renders shared card and empty-state surfaces without animation wrappers", () => {
    const { container } = render(
      <Card>
        <EmptyState icon={Circle} title="No results" />
      </Card>
    );
    expect(
      screen.getByRole("heading", { name: "No results" })
    ).toBeInTheDocument();
    expect(
      container.querySelector('[style*="transform"]')
    ).not.toBeInTheDocument();
  });
});
