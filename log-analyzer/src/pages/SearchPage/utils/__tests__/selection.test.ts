import type { LogEntry } from "../../../../types/api-responses";
import { findSelectedLog } from "../selection";

const firstEntry = { id: 0, content: "first" } as LogEntry;

describe("findSelectedLog", () => {
  it("returns the result whose id is zero", () => {
    expect(findSelectedLog(0, new Map([[0, firstEntry]]))).toBe(firstEntry);
  });

  it("returns undefined only when the selection is null", () => {
    expect(findSelectedLog(null, new Map([[0, firstEntry]]))).toBeUndefined();
  });
});
