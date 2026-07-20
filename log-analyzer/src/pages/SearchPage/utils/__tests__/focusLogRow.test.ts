import { restoreLogRowFocus } from "../focusLogRow";

describe("restoreLogRowFocus", () => {
  it("returns focus to the log row that opened the inspector", () => {
    window.requestAnimationFrame = jest.fn((callback: FrameRequestCallback) => {
      callback(0);
      return 1;
    });
    const row = document.createElement("button");
    row.dataset.logId = "42";
    document.body.appendChild(row);
    restoreLogRowFocus(42);
    expect(row).toHaveFocus();
    row.remove();
  });
});
